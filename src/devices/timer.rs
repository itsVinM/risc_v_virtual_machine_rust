use super::Device;

// CLINT register offsets relative to CLINT_BASE
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

    // Override defaults: CLINT has proper 64-bit registers.
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
