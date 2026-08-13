#ifndef KERNEL_MEM_H
#define KERNEL_MEM_H

void *kmalloc(unsigned long n);
void *kzalloc(unsigned long n);
unsigned long kmem_used(void);
unsigned long kmem_capacity(void);
unsigned long kmem_allocs(void);

#endif
