// Memory and bus boundary tests
use riscv_core::Bus;
use riscv_core::bus::{DRAM_BASE, DRAM_END, CLINT_BASE};
use riscv_core::traps::TrapCause;

fn empty_bus() -> Bus { Bus::new(&[0u8; 64]) }

#[test]
fn dram_read_write_roundtrip() {
    let mut bus = empty_bus();
    bus.write64(DRAM_BASE, 0xDEAD_BEEF_CAFE_BABEu64).unwrap();
    assert_eq!(bus.read64(DRAM_BASE).unwrap(), 0xDEAD_BEEF_CAFE_BABEu64);
}

#[test]
fn dram_byte_granularity() {
    let mut bus = empty_bus();
    bus.write8(DRAM_BASE, 0xAB).unwrap();
    bus.write8(DRAM_BASE + 1, 0xCD).unwrap();
    assert_eq!(bus.read16(DRAM_BASE).unwrap(), 0xCDAB);
}

#[test]
fn oob_read_returns_fault() {
    let bus = empty_bus();
    assert_eq!(bus.read8(0x0000_0000), Err(TrapCause::LoadAccessFault));
    assert_eq!(bus.read8(DRAM_END + 1), Err(TrapCause::LoadAccessFault));
    assert_eq!(bus.read32(0xFFFF_FFFF_FFFF_0000u64), Err(TrapCause::LoadAccessFault));
}

#[test]
fn oob_write_returns_fault() {
    let mut bus = empty_bus();
    assert_eq!(bus.write8(0x0000_0001, 0xFF), Err(TrapCause::StoreAccessFault));
}

#[test]
fn clint_mtime_advances() {
    let mut bus = empty_bus();
    let t0 = bus.clint.mtime;
    bus.tick();
    bus.tick();
    bus.tick();
    assert_eq!(bus.clint.mtime, t0 + 3);
}

#[test]
fn clint_read_write_64() {
    let mut bus = empty_bus();
    bus.write64(CLINT_BASE + 0x4000, 0x1234_5678_9ABC_DEF0u64).unwrap();
    assert_eq!(bus.read64(CLINT_BASE + 0x4000).unwrap(), 0x1234_5678_9ABC_DEF0u64);
}

#[test]
fn write_then_read_32() {
    let mut bus = empty_bus();
    bus.write32(DRAM_BASE + 8, 0xCAFE_BABEu32).unwrap();
    assert_eq!(bus.read32(DRAM_BASE + 8).unwrap(), 0xCAFE_BABEu32);
}

#[test]
fn partial_overlap_boundary() {
    let mut bus = Bus::new(&[0u8; 128 * 1024 * 1024]);
    let near_end = DRAM_END - 8;
    bus.write64(near_end, 0xFFFF_FFFF_FFFF_FFFFu64).unwrap();
    assert_eq!(bus.read64(near_end).unwrap(), 0xFFFF_FFFF_FFFF_FFFFu64);
}
