#ifndef SBI_H
#define SBI_H

#include "types.h"

struct sbiret {
    long error;
    long value;
};

struct sbiret sbi_ecall(int ext, int fid,
                        u64 a0, u64 a1, u64 a2,
                        u64 a3, u64 a4, u64 a5);

struct sbiret sbi_set_timer(u64 stime);
struct sbiret sbi_send_ipi(u64 hmask, u64 hbase);
struct sbiret sbi_hart_start(u64 hartid, u64 entry, u64 priv);

#endif
