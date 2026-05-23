// ── Device trait ──────────────────────────────────────────────────────────────
// Common interface for all memory-mapped I/O peripherals.
// Implementors only need read32/write32; byte and 64-bit variants are derived.
pub trait Device {
    fn read32(&self,  offset: u64) -> u32;
    fn write32(&mut self, offset: u64, val: u32);

    fn read8(&self, offset: u64) -> u8 { self.read32(offset) as u8 }
    fn write8(&mut self, offset: u64, val: u8) { self.write32(offset, val as u32); }
    fn read64(&self, offset: u64) -> u64 {
        (self.read32(offset) as u64) | ((self.read32(offset + 4) as u64) << 32)
    }
    fn write64(&mut self, offset: u64, val: u64) {
        self.write32(offset, val as u32);
        self.write32(offset + 4, (val >> 32) as u32);
    }
}

// ── CLINT — Core Local Interruptor (timer) ────────────────────────────────────
// mtime increments every tick; fires a timer interrupt when mtime >= mtimecmp.
const MTIMECMP_LO: u64 = 0x4000;
const MTIMECMP_HI: u64 = 0x4004;
const MTIME_LO:    u64 = 0xBFF8;
const MTIME_HI:    u64 = 0xBFFC;

pub struct Clint {
    pub mtime:    u64,
    pub mtimecmp: u64,
}

impl Clint {
    pub fn new() -> Self { Self { mtime: 0, mtimecmp: u64::MAX } }
    pub fn tick(&mut self) { self.mtime = self.mtime.wrapping_add(1); }
    pub fn timer_pending(&self) -> bool { self.mtime >= self.mtimecmp }
}

impl Device for Clint {
    fn read32(&self, offset: u64) -> u32 {
        match offset {
            MTIMECMP_LO => self.mtimecmp as u32,
            MTIMECMP_HI => (self.mtimecmp >> 32) as u32,
            MTIME_LO    => self.mtime as u32,
            MTIME_HI    => (self.mtime >> 32) as u32,
            _           => 0,
        }
    }

    fn write32(&mut self, offset: u64, val: u32) {
        match offset {
            MTIMECMP_LO => self.mtimecmp = (self.mtimecmp & 0xFFFF_FFFF_0000_0000) | val as u64,
            MTIMECMP_HI => self.mtimecmp = (self.mtimecmp & 0x0000_0000_FFFF_FFFF) | ((val as u64) << 32),
            MTIME_LO    => self.mtime    = (self.mtime    & 0xFFFF_FFFF_0000_0000) | val as u64,
            MTIME_HI    => self.mtime    = (self.mtime    & 0x0000_0000_FFFF_FFFF) | ((val as u64) << 32),
            _           => {}
        }
    }

    fn read64(&self, offset: u64) -> u64 {
        match offset {
            MTIMECMP_LO => self.mtimecmp,
            MTIME_LO    => self.mtime,
            _           => 0,
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

// ── PLIC — Platform Level Interrupt Controller (minimal stub) ─────────────────
pub struct Plic {
    priority:  [u32; 53],
    pending:   u64,
    enabled:   u64,
    threshold: u32,
}

impl Plic {
    pub fn new() -> Self {
        Self { priority: [0u32; 53], pending: 0, enabled: 0, threshold: 0 }
    }

    pub fn raise(&mut self, irq: u32) {
        if irq > 0 && irq < 64 { self.pending |= 1u64 << (irq - 1); }
    }

    fn claim(&self) -> u32 {
        let active = self.pending & self.enabled;
        if active == 0 { 0 } else { active.trailing_zeros() + 1 }
    }

    fn complete(&mut self, irq: u32) {
        if irq > 0 && irq < 64 { self.pending &= !(1u64 << (irq - 1)); }
    }
}

impl Device for Plic {
    fn read32(&self, offset: u64) -> u32 {
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

    fn write32(&mut self, offset: u64, val: u32) {
        match offset {
            0x000..=0x0D0 => { let i = (offset / 4) as usize; if i < 53 { self.priority[i] = val; } }
            0x2000        => self.enabled = (self.enabled & 0xFFFF_FFFF_0000_0000) | val as u64,
            0x2004        => self.enabled = (self.enabled & 0x0000_0000_FFFF_FFFF) | ((val as u64) << 32),
            0x20_0000     => self.threshold = val,
            0x20_0004     => self.complete(val),
            _             => {}
        }
    }
}
