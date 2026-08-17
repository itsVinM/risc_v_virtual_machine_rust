#ifndef CSR_H
#define CSR_H

#include "types.h"

#define CSR_READ(csr) ({                \
    u64 __v;                            \
    asm volatile("csrr %0, " #csr       \
                 : "=r"(__v));          \
    __v;                                \
})

#define CSR_WRITE(csr, val)             \
    asm volatile("csrw " #csr ", %0"    \
                 :: "r"((u64)(val))     \
                 : "memory")

#define CSR_SET(csr, bits)              \
    asm volatile("csrs " #csr ", %0"    \
                 :: "r"((u64)(bits))    \
                 : "memory")

#define CSR_CLEAR(csr, bits)            \
    asm volatile("csrc " #csr ", %0"    \
                 :: "r"((u64)(bits))    \
                 : "memory")

#endif
