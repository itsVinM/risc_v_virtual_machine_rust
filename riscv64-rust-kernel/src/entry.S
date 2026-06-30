.section .text.entry, "ax"
.global _entry

_entry:
    # a0 = hartid, a1 = DTB address (from OpenSBI)

    # Disable interrupts
    csrw sie, zero

    # Set trap vector
    la t0, trap_vector
    csrw stvec, t0

    # Set stack pointer
    la sp, _stack_top

    # Clear BSS
    la t0, _bss_start
    la t1, _bss_end
1:  bgeu t0, t1, 2f
    sw zero, 0(t0)
    addi t0, t0, 4
    j 1b
2:

    # Jump to Rust main with hartid in a0, dtb in a1
    jal ra, kernel_main

    # Spin if returns
    j .
