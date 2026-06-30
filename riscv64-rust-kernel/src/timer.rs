use crate::sbi;

const CLINT_MTIME: *mut u64 = 0x0200_BFF8 as *mut u64;
const TICKS_PER_SEC: u64 = 10_000_000;

static mut TICKS: u64 = 0;

pub fn read_mtime() -> u64 {
    unsafe { core::ptr::read_volatile(CLINT_MTIME) }
}

pub fn set_next_timer() {
    let now = read_mtime();
    sbi::set_timer(now + TICKS_PER_SEC / 100);
}

pub fn tick() {
    unsafe {
        TICKS = TICKS.wrapping_add(1);
    }
}

pub fn uptime_ms() -> u64 {
    let ticks = unsafe { TICKS };
    ticks * 10
}

pub fn init() {
    set_next_timer();
}
