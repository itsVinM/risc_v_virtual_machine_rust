use super::Device;

// PLIC — Platform Level Interrupt Controller (minimal stub)
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
            0x000..=0x0D0 => {
                let idx = (offset / 4) as usize;
                if idx < 53 { self.priority[idx] } else { 0 }
            }
            0x1000 => self.pending as u32,
            0x1004 => (self.pending >> 32) as u32,
            0x2000 => self.enabled as u32,
            0x2004 => (self.enabled >> 32) as u32,
            0x20_0000 => self.threshold,
            0x20_0004 => self.claim(),
            _ => 0,
        }
    }

    fn write32(&mut self, offset: u64, val: u32) {
        match offset {
            0x000..=0x0D0 => {
                let idx = (offset / 4) as usize;
                if idx < 53 { self.priority[idx] = val; }
            }
            0x2000 => self.enabled = (self.enabled & 0xFFFF_FFFF_0000_0000) | val as u64,
            0x2004 => self.enabled = (self.enabled & 0x0000_0000_FFFF_FFFF) | ((val as u64) << 32),
            0x20_0000 => self.threshold = val,
            0x20_0004 => self.complete(val),
            _ => {}
        }
    }
}
