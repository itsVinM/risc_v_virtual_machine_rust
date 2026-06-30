use crate::allocator;
use crate::println;
use core::sync::atomic::{AtomicBool, Ordering};

const MMIO: usize = 0x1000_1000;

#[inline(always)]
fn rd32(a: usize) -> u32 { unsafe { core::ptr::read_volatile(a as *const u32) } }
#[inline(always)]
fn wr32(a: usize, v: u32) { unsafe { core::ptr::write_volatile(a as *mut u32, v) } }

const QS: usize = 16;

#[repr(C, packed)]
struct Desc { addr: u64, len: u32, flags: u16, next: u16 }

#[repr(C, packed)]
struct Avail { flags: u16, idx: u16, ring: [u16; QS] }

#[repr(C, packed)]
struct UsedElem { id: u32, len: u32 }

#[repr(C, packed)]
struct Used { flags: u16, idx: u16, ring: [UsedElem; QS] }

#[repr(C, packed)]
struct BlkReq { type_: u32, reserved: u32, sector: u64 }

static IRQ: AtomicBool = AtomicBool::new(false);
static mut READY: bool = false;
static mut BASE: usize = 0;

pub fn init() {
    let m = rd32(MMIO);
    let v = rd32(MMIO + 0x004);
    let d = rd32(MMIO + 0x008);
    println!("  virtio: magic=0x{:x} ver={} dev={}", m, v, d);
    if m != 0x74726976 { println!("  no device"); return; }

    wr32(MMIO + 0x070, 1);
    wr32(MMIO + 0x070, 3);

    wr32(MMIO + 0x020, 0);
    wr32(MMIO + 0x070, 0xB);
    if rd32(MMIO + 0x070) & 8 == 0 { println!("  feat NAK"); wr32(MMIO + 0x070, 0x80); return; }

    wr32(MMIO + 0x030, 0);
    let qm = rd32(MMIO + 0x034);
    let qn = if qm > QS as u32 { QS as u32 } else { qm };
    wr32(MMIO + 0x038, qn);
    wr32(MMIO + 0x03C, 64);

    let dsz = core::mem::size_of::<Desc>() * qn as usize;
    let asz = core::mem::size_of::<Avail>();
    let usz = core::mem::size_of::<Used>();
    let total = dsz + asz + usz;
    let np = (total + 4095) / 4096;

    let pa = match allocator::alloc_pages(np) { Some(p) => p, None => { println!("  OOM"); return; } };
    unsafe { core::ptr::write_bytes(pa as *mut u8, 0, np * 4096); }

    wr32(MMIO + 0x040, (pa >> 12) as u32);
    wr32(MMIO + 0x070, 0x1F);

    unsafe { BASE = pa; READY = true; }
    println!("  ready np={} pa=0x{:x}", np, pa);
}

pub fn handle_irq() {
    let isr = rd32(MMIO + 0x060);
    if isr & 1 != 0 { IRQ.store(true, Ordering::SeqCst); wr32(MMIO + 0x064, 1); }
}

pub fn read_sector(sector: u64, buf: &mut [u8; 512]) -> bool {
    unsafe { if !READY { return false; } }
    do_rw(sector, buf.as_mut_ptr(), 0)
}

pub fn write_sector(sector: u64, buf: &[u8; 512]) -> bool {
    unsafe { if !READY { return false; } }
    do_rw(sector, buf.as_ptr() as *mut u8, 1)
}

fn do_rw(sector: u64, buf: *mut u8, type_: u32) -> bool {
    let base = unsafe { BASE };
    let qn = rd32(MMIO + 0x038) as usize;

    let rp = match allocator::alloc_page() { Some(p) => p, None => return false };
    let sp = match allocator::alloc_page() { Some(p) => p, None => { allocator::free_page(rp); return false } };

    unsafe { core::ptr::write_volatile(rp as *mut BlkReq, BlkReq { type_, reserved: 0, sector }) }

    // Descriptors at base+0, base+16, base+32
    let d = base as *mut Desc;
    unsafe {
        core::ptr::write_volatile(d, Desc { addr: rp as u64, len: 16, flags: 1, next: 1 });
        let wf: u16 = if type_ == 0 { 2 } else { 0 };
        core::ptr::write_volatile(d.add(1), Desc { addr: buf as u64, len: 512, flags: 1 | wf, next: 2 });
        core::ptr::write_volatile(d.add(2), Desc { addr: sp as u64, len: 1, flags: 2, next: 0 });
    }

    IRQ.store(false, Ordering::SeqCst);

    // Avail ring at base + dsz
    let avail = (base + core::mem::size_of::<Desc>() * qn) as *mut Avail;
    unsafe {
        let ai = (*avail).idx;
        if ai as usize % QS < QS {
            (*avail).ring[ai as usize % QS] = 0;
        }
        core::sync::atomic::fence(Ordering::SeqCst);
        (*avail).idx = ai.wrapping_add(1);
    }

    wr32(MMIO + 0x050, 0);

    for _ in 0..10_000_000 {
        if IRQ.load(Ordering::SeqCst) { break; }
        unsafe { core::arch::asm!("nop"); }
    }

    let ok = unsafe { core::ptr::read_volatile(sp as *const u8) == 0 };
    allocator::free_page(rp);
    allocator::free_page(sp);
    ok
}
