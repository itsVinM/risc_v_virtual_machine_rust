#include "sbi.h"

#define SBI_EXT_TIME  0x54494D45
#define SBI_EXT_IPI   0x735049
#define SBI_EXT_HSM   0x48534D

#define SBI_FID_SET_TIMER 0
#define SBI_FID_SEND_IPI  0
#define SBI_FID_HART_START 0

struct sbiret sbi_call(uint64_t ext, uint64_t fid,
                       uint64_t a0, uint64_t a1, uint64_t a2)
{
    register uint64_t a0_ asm("a0") = a0;
    register uint64_t a1_ asm("a1") = a1;
    register uint64_t a2_ asm("a2") = a2;
    register uint64_t a7_ asm("a7") = ext;
    register uint64_t a6_ asm("a6") = fid;

    asm volatile(
        "ecall"
        : "+r"(a0_), "+r"(a1_)
        : "r"(a2_), "r"(a7_), "r"(a6_)
        : "memory"
    );

    return (struct sbiret){ a0_, a1_ };
}

void sbi_set_timer(uint64_t stime)
{
    sbi_call(SBI_EXT_TIME, SBI_FID_SET_TIMER, stime, 0, 0);
}

void sbi_send_ipi(uint64_t hmask)
{
    sbi_call(SBI_EXT_IPI, SBI_FID_SEND_IPI, hmask, 0, 0);
}

void sbi_hart_start(uint64_t hartid, uint64_t entry, uint64_t priv)
{
    sbi_call(SBI_EXT_HSM, SBI_FID_HART_START, hartid, entry, priv);
}
