#include "mem.h"
#include "printf.h"

#define HEAP_SIZE (64u * 1024u)

static unsigned char heap[HEAP_SIZE] __attribute__((aligned(16)));
static unsigned long used;
static unsigned long allocs;

void *kmalloc(unsigned long n)
{
    unsigned long aligned;
    if (n == 0)
        n = 1;
    aligned = (n + 15u) & ~15ul;
    if (used + aligned > HEAP_SIZE) {
        printf("kmalloc: out of memory (%lu bytes requested)\n", n);
        return 0;
    }
    {
        void *p = heap + used;
        used += aligned;
        allocs++;
        return p;
    }
}

void *kzalloc(unsigned long n)
{
    void *p = kmalloc(n);
    if (p) {
        unsigned char *q = p;
        unsigned long i;
        for (i = 0; i < n; i++)
            q[i] = 0;
    }
    return p;
}

unsigned long kmem_used(void)
{
    return used;
}

unsigned long kmem_capacity(void)
{
    return HEAP_SIZE;
}

unsigned long kmem_allocs(void)
{
    return allocs;
}
