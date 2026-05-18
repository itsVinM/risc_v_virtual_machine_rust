pub mod plic;
pub mod timer;

/// Common interface for all memory-mapped I/O peripherals.
///
/// Implementors only need to provide `read32` and `write32`.
/// The byte and 64-bit variants are derived from those two by default.
pub trait Device {
    fn read32(&self, offset: u64) -> u32;
    fn write32(&mut self, offset: u64, val: u32);

    fn read8(&self, offset: u64) -> u8 {
        self.read32(offset) as u8
    }
    fn write8(&mut self, offset: u64, val: u8) {
        self.write32(offset, val as u32);
    }
    fn read64(&self, offset: u64) -> u64 {
        (self.read32(offset) as u64) | ((self.read32(offset + 4) as u64) << 32)
    }
    fn write64(&mut self, offset: u64, val: u64) {
        self.write32(offset, val as u32);
        self.write32(offset + 4, (val >> 32) as u32);
    }
}
