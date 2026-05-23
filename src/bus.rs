use crate::devices::{Clint, Device, Plic};
use crate::traps::TrapCause;

// ── Memory map ────────────────────────────────────────────────────────────────
pub const DRAM_BASE:  u64 = 0x8000_0000;
pub const DRAM_END:   u64 = 0x8800_0000; // 128 MB
pub const CLINT_BASE: u64 = 0x0200_0000;
pub const CLINT_END:  u64 = 0x020F_FFFF;
pub const PLIC_BASE:  u64 = 0x0C00_0000;
pub const PLIC_END:   u64 = 0x0FFF_FFFF;

const DRAM_SIZE: u64 = 128 * 1024 * 1024;

// ── DRAM ──────────────────────────────────────────────────────────────────────
struct Memory {
    data: Vec<u8>,
}

impl Memory {
    fn new() -> Self {
        Self { data: vec![0u8; DRAM_SIZE as usize] }
    }

    fn load_binary(bytes: &[u8]) -> Self {
        let mut m = Self::new();
        let len = bytes.len().min(DRAM_SIZE as usize);
        m.data[..len].copy_from_slice(&bytes[..len]);
        m
    }

    fn into_dram(mut data: Vec<u8>) -> Self {
        data.resize(DRAM_SIZE as usize, 0);
        Self { data }
    }

    #[inline(always)]
    fn off(&self, addr: u64) -> Option<usize> {
        if addr >= DRAM_BASE && addr < DRAM_BASE + DRAM_SIZE {
            Some((addr - DRAM_BASE) as usize)
        } else {
            None
        }
    }

    fn read8(&self,  addr: u64) -> Option<u8>  { Some(self.data[self.off(addr)?]) }
    fn read32(&self, addr: u64) -> Option<u32> {
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

// ── Bus ───────────────────────────────────────────────────────────────────────
pub struct Bus {
    dram:  Memory,
    pub clint: Clint,
    pub plic:  Plic,
}

impl Bus {
    pub fn new(binary: &[u8]) -> Self {
        Self { dram: Memory::load_binary(binary), clint: Clint::new(), plic: Plic::new() }
    }

    pub fn from_dram(data: Vec<u8>) -> Self {
        Self { dram: Memory::into_dram(data), clint: Clint::new(), plic: Plic::new() }
    }

    pub fn tick(&mut self) { self.clint.tick(); }

    pub fn read8(&self, addr: u64) -> Result<u8, TrapCause> {
        match addr {
            DRAM_BASE..=DRAM_END   => self.dram.read8(addr).ok_or(TrapCause::LoadAccessFault),
            CLINT_BASE..=CLINT_END => Ok(self.clint.read8(addr - CLINT_BASE)),
            PLIC_BASE..=PLIC_END   => Ok(self.plic.read8(addr - PLIC_BASE)),
            _                      => Err(TrapCause::LoadAccessFault),
        }
    }

    pub fn read16(&self, addr: u64) -> Result<u16, TrapCause> {
        let lo = self.read8(addr)? as u16;
        let hi = self.read8(addr + 1)? as u16;
        Ok(lo | (hi << 8))
    }

    pub fn read32(&self, addr: u64) -> Result<u32, TrapCause> {
        match addr {
            DRAM_BASE..=DRAM_END   => self.dram.read32(addr).ok_or(TrapCause::LoadAccessFault),
            CLINT_BASE..=CLINT_END => Ok(self.clint.read32(addr - CLINT_BASE)),
            PLIC_BASE..=PLIC_END   => Ok(self.plic.read32(addr - PLIC_BASE)),
            _                      => Err(TrapCause::LoadAccessFault),
        }
    }

    pub fn read64(&self, addr: u64) -> Result<u64, TrapCause> {
        match addr {
            DRAM_BASE..=DRAM_END   => self.dram.read64(addr).ok_or(TrapCause::LoadAccessFault),
            CLINT_BASE..=CLINT_END => Ok(self.clint.read64(addr - CLINT_BASE)),
            _ => {
                let lo = self.read32(addr)? as u64;
                let hi = self.read32(addr + 4)? as u64;
                Ok(lo | (hi << 32))
            }
        }
    }

    pub fn write8(&mut self, addr: u64, val: u8) -> Result<(), TrapCause> {
        match addr {
            DRAM_BASE..=DRAM_END   => { self.dram.write8(addr, val); Ok(()) }
            CLINT_BASE..=CLINT_END => { self.clint.write8(addr - CLINT_BASE, val); Ok(()) }
            PLIC_BASE..=PLIC_END   => { self.plic.write8(addr - PLIC_BASE, val); Ok(()) }
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
            CLINT_BASE..=CLINT_END => { self.clint.write32(addr - CLINT_BASE, val); Ok(()) }
            PLIC_BASE..=PLIC_END   => { self.plic.write32(addr - PLIC_BASE, val); Ok(()) }
            _                      => Err(TrapCause::StoreAccessFault),
        }
    }

    pub fn write64(&mut self, addr: u64, val: u64) -> Result<(), TrapCause> {
        match addr {
            DRAM_BASE..=DRAM_END   => { self.dram.write64(addr, val); Ok(()) }
            CLINT_BASE..=CLINT_END => { self.clint.write64(addr - CLINT_BASE, val); Ok(()) }
            _ => {
                self.write32(addr, val as u32)?;
                self.write32(addr + 4, (val >> 32) as u32)
            }
        }
    }
}
