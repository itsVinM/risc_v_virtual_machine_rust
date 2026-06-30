use core::arch::global_asm;

global_asm!(
    ".section .text.entry, \"ax\"",
    ".global _entry",
    "_entry:",
    "   csrw sie, zero",
    "   la t0, trap_vector",
    "   csrw stvec, t0",
    "   la sp, _stack_top",
    "   la t0, _bss_start",
    "   la t1, _bss_end",
    "1: bgeu t0, t1, 2f",
    "   sw zero, 0(t0)",
    "   addi t0, t0, 4",
    "   j 1b",
    "2:",
    "   jal ra, kernel_main",
    "   j .",
);

extern "C" {
    pub fn kernel_main(hartid: u64, dtb: u64);
    fn trap_vector();
}
