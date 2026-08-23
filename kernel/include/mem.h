#ifndef KERNEL_MEM_H
#define KERNEL_MEM_H

#include <stddef.h>

void *kmalloc(size_t n);
void *kzalloc(size_t n);
void  kfree(void *p);
size_t kmem_used(void);
size_t kmem_capacity(void);
size_t kmem_allocs(void);

#endif
