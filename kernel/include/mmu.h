#ifndef MMU_H
#define MMU_H

#include "types.h"

void mmu_init(void);
u64 *mmu_root(void);

#endif
