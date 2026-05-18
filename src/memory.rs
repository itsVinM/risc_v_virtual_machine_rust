extern crate alloc;
use alloc::vec;
use alloc::vec::Vec;

pub const DRAM_BASE: u64 = 0x8000_0000;
pub const DRAM_SIZE: u64 = 128 * 1024 * 1024;

pub struct Memory {
    data: Vec<u8>,
}

impl Memory {
    pub fn new() -> Self {
        Self { data: vec![0u8; DRAM_SIZE as usize] }
    }

    pub fn load_binary(bytes: &[u8]) -> Self {
        let mut m = Self::new();
        let len = bytes.len().min(DRAM_SIZE as usize);
        m.data[..len].copy_from_slice(&bytes[..len]);
        m
    }

    #[inline(always)]
    fn off(&self, addr: u64) -> Option<usize> {
        if addr >= DRAM_BASE && addr < DRAM_BASE + DRAM_SIZE {
            Some((addr - DRAM_BASE) as usize)
        } else {
            None
        }
    }

    pub fn read8(&self, addr: u64)  -> Option<u8>  { Some(self.data[self.off(addr)?]) }
    pub fn read16(&self, addr: u64) -> Option<u16> {
        let i = self.off(addr)?;
        Some(u16::from_le_bytes(self.data[i..i+2].try_into().ok()?))
    }
    pub fn read32(&self, addr: u64) -> Option<u32> {
        let i = self.off(addr)?;
        Some(u32::from_le_bytes(self.data[i..i+4].try_into().ok()?))
    }
    pub fn read64(&self, addr: u64) -> Option<u64> {
        let i = self.off(addr)?;
        Some(u64::from_le_bytes(self.data[i..i+8].try_into().ok()?))
    }

    pub fn write8(&mut self, addr: u64, v: u8) {
        if let Some(i) = self.off(addr) { self.data[i] = v; }
    }
    pub fn write16(&mut self, addr: u64, v: u16) {
        if let Some(i) = self.off(addr) { self.data[i..i+2].copy_from_slice(&v.to_le_bytes()); }
    }
    pub fn write32(&mut self, addr: u64, v: u32) {
        if let Some(i) = self.off(addr) { self.data[i..i+4].copy_from_slice(&v.to_le_bytes()); }
    }
    pub fn write64(&mut self, addr: u64, v: u64) {
        if let Some(i) = self.off(addr) { self.data[i..i+8].copy_from_slice(&v.to_le_bytes()); }
    }

    pub fn slice(&self, addr: u64, len: usize) -> Option<&[u8]> {
        let i = self.off(addr)?;
        if i + len <= self.data.len() { Some(&self.data[i..i+len]) } else { None }
    }
}
