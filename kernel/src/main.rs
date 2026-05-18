//! Bare-metal RV64I kernel demonstrating M-mode timer interrupts.
//!
//! Flow:
//!   1. Install trap vector
//!   2. Arm CLINT timer (mtimecmp = mtime + 500)
//!   3. Enable timer interrupts (mie.MTIE, mstatus.MIE)
//!   4. Spin until TICKS reaches 5
//!   5. ebreak → VM prints  a0=5

#![no_std]
#![no_main]

use core::arch::global_asm;
use core::panic::PanicInfo;

global_asm!(r#"
    .option norvc               /* disable compressed instructions */
    .section .text.init
    .global  _start
_start:
    # Stack near top of DRAM (0x8800_0000 - 16)
    li      sp, 0x87FFFFF0

    # Install trap handler (direct mode — address must be 4-aligned)
    la      t0, _trap
    csrw    mtvec, t0

    # Arm first timer: mtimecmp = mtime + 500
    li      t0, 0x200BFF8      # CLINT MTIME  (0x0200_BFF8)
    ld      t1, 0(t0)
    li      t2, 500
    add     t1, t1, t2
    li      t0, 0x2004000      # CLINT MTIMECMP (0x0200_4000)
    sd      t1, 0(t0)

    # Enable machine timer interrupt: mie.MTIE (bit 7)
    li      t0, 0x80
    csrs    mie, t0

    # Enable global interrupts: mstatus.MIE (bit 3)
    li      t0, 0x8
    csrs    mstatus, t0

    # Spin until TICKS >= 5
.Lloop:
    la      t0, TICKS
    ld      t1, 0(t0)
    li      t2, 5
    blt     t1, t2, .Lloop

    # a0 = tick count, then halt
    mv      a0, t1
    ebreak


    # ── Timer trap handler ────────────────────────────────────────────
    .section .text
    .align  2
    .global _trap
_trap:
    # Save registers we clobber
    addi    sp, sp, -32
    sd      t0,  0(sp)
    sd      t1,  8(sp)
    sd      t2, 16(sp)
    sd      ra, 24(sp)

    # Re-arm: mtimecmp = mtime + 500
    li      t0, 0x200BFF8
    ld      t1, 0(t0)
    li      t2, 500
    add     t1, t1, t2
    li      t0, 0x2004000
    sd      t1, 0(t0)

    # TICKS++
    la      t0, TICKS
    ld      t1, 0(t0)
    addi    t1, t1, 1
    sd      t1, 0(t0)

    # Restore and return from trap
    ld      t0,  0(sp)
    ld      t1,  8(sp)
    ld      t2, 16(sp)
    ld      ra, 24(sp)
    addi    sp, sp, 32
    mret


    # ── Data ──────────────────────────────────────────────────────────
    .section .data
    .align  8
    .global TICKS
TICKS:
    .quad   0
"#);

#[panic_handler]
fn panic(_: &PanicInfo) -> ! {
    loop { unsafe { core::arch::asm!("ebreak"); } }
}
