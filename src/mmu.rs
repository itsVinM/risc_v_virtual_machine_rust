use crate::traps::TrapCause;

// Memory map 
pub const DRAM_BASE:  u64 = 0x8000_0000;
pub const DRAM_END:   u64 = 0x8800_0000;
pub const CLINT_BASE: u64 = 0x0200_0000;
pub const CLINT_END:  u64 = 0x020F_FFFF;
pub const PLIC_BASE:  u64 = 0x0C00_0000;
pub const PLIC_END:   u64 = 0x0FFF_FFFF;

const DRAM_SIZE: u64 = 128 * 1024 * 1024;

// DRAM 
struct Memory {
    data: Vec<u8>,
}

impl Memory {
    fn new() -> Self { Self { data: vec![0u8; DRAM_SIZE as usize] } }

    fn load_binary(bytes: &[u8]) -> Self {
        let mut m = Self::new();
        m.data[..bytes.len().min(DRAM_SIZE as usize)]
            .copy_from_slice(&bytes[..bytes.len().min(DRAM_SIZE as usize)]);
        m
    }

    fn into_dram(mut data: Vec<u8>) -> Self {
        data.resize(DRAM_SIZE as usize, 0);
        Self { data }
    }

    #[inline(always)]
    fn off(&self, addr: u64) -> Option<usize> {
        (addr >= DRAM_BASE && addr < DRAM_BASE + DRAM_SIZE)
            .then(|| (addr - DRAM_BASE) as usize)
    }

    fn read8(&self,  addr: u64) -> Option<u8>  { Some(self.data[self.off(addr)?]) }
    fn read32(&self, addr: u64) -> Option<u32>  {
        let i = self.off(addr)?;
        Some(u32::from_le_bytes(self.data[i..i+4].try_into().ok()?))
    }
    fn read64(&self, addr: u64) -> Option<u64> {
        let i = self.off(addr)?;
        Some(u64::from_le_bytes(self.data[i..i+8].try_into().ok()?))
    }
    fn write8(&mut self,  addr: u64, v: u8)  { if let Some(i) = self.off(addr) { self.data[i] = v; } }
    fn write32(&mut self, addr: u64, v: u32) { if let Some(i) = self.off(addr) { self.data[i..i+4].copy_from_slice(&v.to_le_bytes()); } }
    fn write64(&mut self, addr: u64, v: u64) { if let Some(i) = self.off(addr) { self.data[i..i+8].copy_from_slice(&v.to_le_bytes()); } }
}

// CLINT — timer
// mtime increments every tick; fires when mtime >= mtimecmp.
const MTIMECMP_LO: u64 = 0x4000;
const MTIMECMP_HI: u64 = 0x4004;
const MTIME_LO:    u64 = 0xBFF8;
const MTIME_HI:    u64 = 0xBFFC;

pub struct Clint {
    pub mtime:    u64,
    pub mtimecmp: u64,
}

impl Clint {
    fn new() -> Self { Self { mtime: 0, mtimecmp: u64::MAX } }
    pub fn timer_pending(&self) -> bool { self.mtime >= self.mtimecmp }

    fn read(&self, offset: u64) -> u32 {
        match offset {
            MTIMECMP_LO => self.mtimecmp as u32,
            MTIMECMP_HI => (self.mtimecmp >> 32) as u32,
            MTIME_LO    => self.mtime as u32,
            MTIME_HI    => (self.mtime >> 32) as u32,
            _           => 0,
        }
    }

    fn read64(&self, offset: u64) -> u64 {
        match offset {
            MTIMECMP_LO => self.mtimecmp,
            MTIME_LO    => self.mtime,
            _           => 0,
        }
    }

    fn write(&mut self, offset: u64, val: u32) {
        match offset {
            MTIMECMP_LO => self.mtimecmp = (self.mtimecmp & 0xFFFF_FFFF_0000_0000) | val as u64,
            MTIMECMP_HI => self.mtimecmp = (self.mtimecmp & 0x0000_0000_FFFF_FFFF) | ((val as u64) << 32),
            MTIME_LO    => self.mtime    = (self.mtime    & 0xFFFF_FFFF_0000_0000) | val as u64,
            MTIME_HI    => self.mtime    = (self.mtime    & 0x0000_0000_FFFF_FFFF) | ((val as u64) << 32),
            _           => {}
        }
    }

    fn write64(&mut self, offset: u64, val: u64) {
        match offset {
            MTIMECMP_LO => self.mtimecmp = val,
            MTIME_LO    => self.mtime    = val,
            _           => {}
        }
    }
}

// PLIC — external interrupt controller (stub) 
struct Plic {
    priority:  [u32; 53],
    pending:   u64,
    enabled:   u64,
    threshold: u32,
}

impl Plic {
    fn new() -> Self { Self { priority: [0; 53], pending: 0, enabled: 0, threshold: 0 } }

    fn claim(&self) -> u32 {
        let active = self.pending & self.enabled;
        if active == 0 { 0 } else { active.trailing_zeros() + 1 }
    }

    fn read(&self, offset: u64) -> u32 {
        match offset {
            0x000..=0x0D0 => { let i = (offset / 4) as usize; if i < 53 { self.priority[i] } else { 0 } }
            0x1000        => self.pending as u32,
            0x1004        => (self.pending >> 32) as u32,
            0x2000        => self.enabled as u32,
            0x2004        => (self.enabled >> 32) as u32,
            0x20_0000     => self.threshold,
            0x20_0004     => self.claim(),
            _             => 0,
        }
    }

    fn write(&mut self, offset: u64, val: u32) {
        match offset {
            0x000..=0x0D0 => { let i = (offset / 4) as usize; if i < 53 { self.priority[i] = val; } }
            0x2000        => self.enabled = (self.enabled & 0xFFFF_FFFF_0000_0000) | val as u64,
            0x2004        => self.enabled = (self.enabled & 0x0000_0000_FFFF_FFFF) | ((val as u64) << 32),
            0x20_0000     => self.threshold = val,
            0x20_0004     => { if val > 0 && val < 64 { self.pending &= !(1u64 << (val - 1)); } }
            _             => {}
        }
    }
}

// MMU / Bus 
pub struct Mmu {
    dram:      Memory,
    pub clint: Clint,
    plic:      Plic,
}

impl Mmu {
    pub fn new(binary: &[u8]) -> Self {
        Self { dram: Memory::load_binary(binary), clint: Clint::new(), plic: Plic::new() }
    }

    pub fn from_dram(data: Vec<u8>) -> Self {
        Self { dram: Memory::into_dram(data), clint: Clint::new(), plic: Plic::new() }
    }

    pub fn tick(&mut self) { self.clint.mtime = self.clint.mtime.wrapping_add(1); }

    pub fn read8(&self, addr: u64) -> Result<u8, TrapCause> {
        match addr {
            DRAM_BASE..=DRAM_END   => self.dram.read8(addr).ok_or(TrapCause::LoadAccessFault),
            CLINT_BASE..=CLINT_END => Ok(self.clint.read(addr - CLINT_BASE) as u8),
            PLIC_BASE..=PLIC_END   => Ok(self.plic.read(addr - PLIC_BASE) as u8),
            _                      => Err(TrapCause::LoadAccessFault),
        }
    }

    pub fn read16(&self, addr: u64) -> Result<u16, TrapCause> {
        Ok(self.read8(addr)? as u16 | ((self.read8(addr + 1)? as u16) << 8))
    }

    pub fn read32(&self, addr: u64) -> Result<u32, TrapCause> {
        match addr {
            DRAM_BASE..=DRAM_END   => self.dram.read32(addr).ok_or(TrapCause::LoadAccessFault),
            CLINT_BASE..=CLINT_END => Ok(self.clint.read(addr - CLINT_BASE)),
            PLIC_BASE..=PLIC_END   => Ok(self.plic.read(addr - PLIC_BASE)),
            _                      => Err(TrapCause::LoadAccessFault),
        }
    }

    pub fn read64(&self, addr: u64) -> Result<u64, TrapCause> {
        match addr {
            DRAM_BASE..=DRAM_END   => self.dram.read64(addr).ok_or(TrapCause::LoadAccessFault),
            CLINT_BASE..=CLINT_END => Ok(self.clint.read64(addr - CLINT_BASE)),
            _ => Ok(self.read32(addr)? as u64 | ((self.read32(addr + 4)? as u64) << 32)),
        }
    }

    pub fn write8(&mut self, addr: u64, val: u8) -> Result<(), TrapCause> {
        match addr {
            DRAM_BASE..=DRAM_END   => { self.dram.write8(addr, val); Ok(()) }
            CLINT_BASE..=CLINT_END => { self.clint.write(addr - CLINT_BASE, val as u32); Ok(()) }
            PLIC_BASE..=PLIC_END   => { self.plic.write(addr - PLIC_BASE, val as u32); Ok(()) }
            _                      => Err(TrapCause::StoreAccessFault),
        }
    }

    pub fn write16(&mut self, addr: u64, val: u16) -> Result<(), TrapCause> {
        self.write8(addr, val as u8)?;
        self.write8(addr + 1, (val >> 8) as u8)
    }

    pub fn write32(&mut self, addr: u64, val: u32) -> Result<(), TrapCause> {
        match addr {
            DRAM_BASE..=DRAM_END   => { self.dram.write32(addr, val); Ok(()) }
            CLINT_BASE..=CLINT_END => { self.clint.write(addr - CLINT_BASE, val); Ok(()) }
            PLIC_BASE..=PLIC_END   => { self.plic.write(addr - PLIC_BASE, val); Ok(()) }
            _                      => Err(TrapCause::StoreAccessFault),
        }
    }

    pub fn write64(&mut self, addr: u64, val: u64) -> Result<(), TrapCause> {
        match addr {
            DRAM_BASE..=DRAM_END   => { self.dram.write64(addr, val); Ok(()) }
            CLINT_BASE..=CLINT_END => { self.clint.write64(addr - CLINT_BASE, val); Ok(()) }
            _ => { self.write32(addr, val as u32)?; self.write32(addr + 4, (val >> 32) as u32) }
        }
    }
}
