#ifndef TRAP_H
#define TRAP_H

#include "types.h"

struct trapframe {
    u64 gpr[31]; /* x1-x31 (x0 is always zero) */
    u64 sstatus;
    u64 sepc;
};

void trap_init(void);
void trap_handler_c(struct trapframe *tf);

#endif
