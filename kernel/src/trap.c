#include "trap.h"
#include "csr.h"
#include "printf.h"
#include "panic.h"

/* All of the register save/restore is in asm volatile — no .S file. */
__attribute__((naked))
void trap_vector(void)
{
    asm volatile(
        /* save all GPRs */
        "addi sp, sp, -33 * 8\n"
        "sd x1,   0 * 8(sp)\n"
        "sd x2,   1 * 8(sp)\n"
        "sd x3,   2 * 8(sp)\n"
        "sd x4,   3 * 8(sp)\n"
        "sd x5,   4 * 8(sp)\n"
        "sd x6,   5 * 8(sp)\n"
        "sd x7,   6 * 8(sp)\n"
        "sd x8,   7 * 8(sp)\n"
        "sd x9,   8 * 8(sp)\n"
        "sd x10,  9 * 8(sp)\n"
        "sd x11, 10 * 8(sp)\n"
        "sd x12, 11 * 8(sp)\n"
        "sd x13, 12 * 8(sp)\n"
        "sd x14, 13 * 8(sp)\n"
        "sd x15, 14 * 8(sp)\n"
        "sd x16, 15 * 8(sp)\n"
        "sd x17, 16 * 8(sp)\n"
        "sd x18, 17 * 8(sp)\n"
        "sd x19, 18 * 8(sp)\n"
        "sd x20, 19 * 8(sp)\n"
        "sd x21, 20 * 8(sp)\n"
        "sd x22, 21 * 8(sp)\n"
        "sd x23, 22 * 8(sp)\n"
        "sd x24, 23 * 8(sp)\n"
        "sd x25, 24 * 8(sp)\n"
        "sd x26, 25 * 8(sp)\n"
        "sd x27, 26 * 8(sp)\n"
        "sd x28, 27 * 8(sp)\n"
        "sd x29, 28 * 8(sp)\n"
        "sd x30, 29 * 8(sp)\n"
        "sd x31, 30 * 8(sp)\n"

        /* save CSRs */
        "csrr a0, sstatus\n"
        "sd a0, 31 * 8(sp)\n"
        "csrr a0, sepc\n"
        "sd a0, 32 * 8(sp)\n"

        /* arg0 = trapframe pointer, call C handler */
        "mv a0, sp\n"
        "call trap_handler_c\n"

        /* restore CSRs */
        "ld a0, 32 * 8(sp)\n"
        "csrw sepc, a0\n"
        "ld a0, 31 * 8(sp)\n"
        "csrw sstatus, a0\n"

        /* restore all GPRs */
        "ld x1,   0 * 8(sp)\n"
        "ld x2,   1 * 8(sp)\n"
        "ld x3,   2 * 8(sp)\n"
        "ld x4,   3 * 8(sp)\n"
        "ld x5,   4 * 8(sp)\n"
        "ld x6,   5 * 8(sp)\n"
        "ld x7,   6 * 8(sp)\n"
        "ld x8,   7 * 8(sp)\n"
        "ld x9,   8 * 8(sp)\n"
        "ld x10,  9 * 8(sp)\n"
        "ld x11, 10 * 8(sp)\n"
        "ld x12, 11 * 8(sp)\n"
        "ld x13, 12 * 8(sp)\n"
        "ld x14, 13 * 8(sp)\n"
        "ld x15, 14 * 8(sp)\n"
        "ld x16, 15 * 8(sp)\n"
        "ld x17, 16 * 8(sp)\n"
        "ld x18, 17 * 8(sp)\n"
        "ld x19, 18 * 8(sp)\n"
        "ld x20, 19 * 8(sp)\n"
        "ld x21, 20 * 8(sp)\n"
        "ld x22, 21 * 8(sp)\n"
        "ld x23, 22 * 8(sp)\n"
        "ld x24, 23 * 8(sp)\n"
        "ld x25, 24 * 8(sp)\n"
        "ld x26, 25 * 8(sp)\n"
        "ld x27, 26 * 8(sp)\n"
        "ld x28, 27 * 8(sp)\n"
        "ld x29, 28 * 8(sp)\n"
        "ld x30, 29 * 8(sp)\n"
        "ld x31, 30 * 8(sp)\n"

        "addi sp, sp, 33 * 8\n"
        "sret\n"
    );
}

/* RISC-V exception codes (S-mode, from privileged spec) */
#define EXC_INST_MISALIGN  0
#define EXC_INST_ACCESS    1
#define EXC_INST_ILLEGAL   2
#define EXC_BREAKPOINT     3
#define EXC_LOAD_MISALIGN  4
#define EXC_LOAD_ACCESS    5
#define EXC_STORE_MISALIGN 6
#define EXC_STORE_ACCESS   7
#define EXC_ECALL_U        8
#define EXC_ECALL_S        9

static const char *exception_name(u64 code)
{
    switch (code) {
    case EXC_INST_MISALIGN:  return "instruction misalign";
    case EXC_INST_ACCESS:    return "instruction access fault";
    case EXC_INST_ILLEGAL:   return "illegal instruction";
    case EXC_BREAKPOINT:     return "breakpoint";
    case EXC_LOAD_MISALIGN:  return "load misalign";
    case EXC_LOAD_ACCESS:    return "load access fault";
    case EXC_STORE_MISALIGN: return "store misalign";
    case EXC_STORE_ACCESS:   return "store access fault";
    case EXC_ECALL_U:        return "ecall U-mode";
    case EXC_ECALL_S:        return "ecall S-mode";
    default:                 return "unknown";
    }
}

void trap_handler_c(struct trapframe *tf)
{
    u64 cause = CSR_READ(scause);
    u64 irq   = cause >> 63;
    u64 code  = cause & 0x7FFFFFFFFFFFFFFFULL;
    u64 stval = CSR_READ(stval);

    if (!irq) {
        printf("trap: exception code=%lu (%s) sepc=0x%lx stval=0x%lx\n",
               code, exception_name(code), tf->sepc, stval);

        /* advance past ecall so we don't loop */
        if (code == EXC_ECALL_S)
            tf->sepc += 4;
        else
            PANIC("unhandled exception");
    } else {
        printf("trap: interrupt code=%lu\n", code);
    }
}

void trap_init(void)
{
    CSR_WRITE(stvec, (u64)trap_vector);
    printf("trap: stvec set to 0x%lx\n", (u64)trap_vector);
}
