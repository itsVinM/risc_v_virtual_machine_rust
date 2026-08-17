#ifndef MEM_H
#define MEM_H

#include "types.h"

void *kmalloc(usize n);
void *kzalloc(usize n);
void  kfree(void *p);
usize kmem_used(void);
usize kmem_capacity(void);
usize kmem_allocs(void);

#endif
