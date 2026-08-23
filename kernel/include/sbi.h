#ifndef KERNEL_SBI_H
#define KERNEL_SBI_H

#include <stdint.h>

struct sbiret {
    uint64_t error;
    uint64_t value;
};

struct sbiret sbi_call(uint64_t ext, uint64_t fid,
                       uint64_t a0, uint64_t a1, uint64_t a2);

void sbi_set_timer(uint64_t stime);
void sbi_send_ipi(uint64_t hmask);
void sbi_hart_start(uint64_t hartid, uint64_t entry, uint64_t priv);

#endif
