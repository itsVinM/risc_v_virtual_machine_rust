#![allow(dead_code)]

const SBI_SET_TIMER: u64 = 0;
const SBI_CONSOLE_PUTCHAR: u64 = 1;
const SBI_CONSOLE_GETCHAR: u64 = 2;
const SBI_SHUTDOWN: u64 = 8;

const SBI_EXT_BASE: u64 = 0x10;
const SBI_EXT_TIMER: u64 = 0x54494D45;
const SBI_EXT_IPI: u64 = 0x735049;
const SBI_EXT_DBCN: u64 = 0x4442434E;

#[inline(always)]
fn sbi_call(ext: u64, fid: u64, arg0: u64, arg1: u64, arg2: u64) -> (u64, u64) {
    let mut error: u64;
    let mut value: u64;
    unsafe {
        core::arch::asm!(
            "ecall",
            in("a7") ext,
            in("a6") fid,
            in("a0") arg0,
            in("a1") arg1,
            in("a2") arg2,
            lateout("a0") error,
            lateout("a1") value,
        );
    }
    (error, value)
}

pub fn legacy_console_putchar(c: u8) {
    sbi_call(SBI_CONSOLE_PUTCHAR, 0, c as u64, 0, 0);
}

pub fn legacy_console_getchar() -> isize {
    let (error, _) = sbi_call(SBI_CONSOLE_GETCHAR, 0, 0, 0, 0);
    error as isize
}

pub fn legacy_shutdown() -> ! {
    sbi_call(SBI_SHUTDOWN, 0, 0, 0, 0);
    loop {}
}

pub fn set_timer(stime_value: u64) {
    sbi_call(SBI_EXT_TIMER, 0, stime_value, 0, 0);
}

pub fn send_ipi(hart_mask: u64) {
    sbi_call(SBI_EXT_IPI, 0, hart_mask, 0, 0);
}

pub fn dbcn_write(buf: &[u8]) -> u64 {
    let (_, value) = sbi_call(SBI_EXT_DBCN, 0, buf.as_ptr() as u64, buf.len() as u64, 0);
    value
}

pub fn dbcn_read(buf: &mut [u8]) -> u64 {
    let (_, value) = sbi_call(SBI_EXT_DBCN, 1, buf.as_ptr() as u64, buf.len() as u64, 0);
    value
}

pub fn dbcn_write_byte(c: u8) {
    sbi_call(SBI_EXT_DBCN, 2, c as u64, 0, 0);
}

pub fn system_reset() -> ! {
    sbi_call(0x53525354, 0, 0, 0, 0);
    loop {}
}

pub fn hart_stop() -> ! {
    sbi_call(0x48534D, 0, 0, 0, 0);
    loop {}
}
