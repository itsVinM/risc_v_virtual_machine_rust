#include "sbi.h"

/* SBI ecall: a7=ext, a6=fid, a0-a5=args. Returns {error, value}. */
struct sbiret sbi_ecall(int ext, int fid,
                        u64 a0, u64 a1, u64 a2,
                        u64 a3, u64 a4, u64 a5)
{
    register u64 t0 asm("a0") = a0;
    register u64 t1 asm("a1") = a1;
    register u64 t2 asm("a2") = a2;
    register u64 t3 asm("a3") = a3;
    register u64 t4 asm("a4") = a4;
    register u64 t5 asm("a5") = a5;
    register u64 t6 asm("a6") = (u64)fid;
    register u64 t7 asm("a7") = (u64)ext;

    asm volatile(
        "ecall"
        : "+r"(t0), "+r"(t1)
        : "r"(t2), "r"(t3), "r"(t4), "r"(t5), "r"(t6), "r"(t7)
        : "memory"
    );

    struct sbiret ret;
    ret.error = (long)t0;
    ret.value = (long)t1;
    return ret;
}

/* Extension IDs */
#define SBI_EXT_TIME 0x54494D45
#define SBI_EXT_IPI  0x735049
#define SBI_EXT_HSM  0x48534D

struct sbiret sbi_set_timer(u64 stime)
{
    return sbi_ecall(SBI_EXT_TIME, 0, stime, 0, 0, 0, 0, 0);
}

struct sbiret sbi_send_ipi(u64 hmask, u64 hbase)
{
    return sbi_ecall(SBI_EXT_IPI, 0, hmask, hbase, 0, 0, 0, 0);
}

struct sbiret sbi_hart_start(u64 hartid, u64 entry, u64 priv)
{
    return sbi_ecall(SBI_EXT_HSM, 0, hartid, entry, priv, 0, 0, 0);
}
