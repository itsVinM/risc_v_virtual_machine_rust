#include "mem.h"
#include "string.h"
#include "printf.h"

#define HEAP_SIZE (64u * 1024u)

struct block {
    usize size;         /* usable bytes (excluding this header) */
    int   free;         /* 1 = free, 0 = allocated */
    struct block *next;
};

#define HDR sizeof(struct block)

static unsigned char heap[HEAP_SIZE] __attribute__((aligned(16)));
static struct block *head = NULL;
static usize heap_offset = 0;   /* bump pointer for extending */
static usize total_allocs = 0;

/* Ensure at least one free block exists, extending the heap if needed. */
static struct block *ensure_block(usize need)
{
    if (heap_offset + HDR + need > HEAP_SIZE)
        return NULL;

    struct block *b = (struct block *)(heap + heap_offset);
    b->size = need;
    b->free = 1;
    b->next = NULL;

    /* append to list */
    if (!head) {
        head = b;
    } else {
        struct block *cur = head;
        while (cur->next)
            cur = cur->next;
        cur->next = b;
    }

    heap_offset += HDR + need;
    return b;
}

static void coalesce(void)
{
    struct block *cur = head;
    while (cur && cur->next) {
        if (cur->free && cur->next->free) {
            cur->size += HDR + cur->next->size;
            cur->next = cur->next->next;
        } else {
            cur = cur->next;
        }
    }
}

void *kmalloc(usize n)
{
    if (n == 0)
        n = 1;

    /* first-fit search */
    struct block *cur = head;
    while (cur) {
        if (cur->free && cur->size >= n) {
            cur->free = 0;
            total_allocs++;
            return (void *)((char *)cur + HDR);
        }
        cur = cur->next;
    }

    /* no fit — extend */
    struct block *b = ensure_block(n);
    if (!b) {
        printf("kmalloc: out of memory (%lu bytes)\n", n);
        return NULL;
    }
    b->free = 0;
    total_allocs++;
    return (void *)((char *)b + HDR);
}

void *kzalloc(usize n)
{
    void *p = kmalloc(n);
    if (p)
        memset(p, 0, n);
    return p;
}

void kfree(void *p)
{
    if (!p)
        return;
    struct block *b = (struct block *)((char *)p - HDR);
    b->free = 1;
    coalesce();
}

usize kmem_used(void)
{
    usize used = 0;
    struct block *cur = head;
    while (cur) {
        if (!cur->free)
            used += cur->size;
        cur = cur->next;
    }
    return used;
}

usize kmem_capacity(void)
{
    return HEAP_SIZE;
}

usize kmem_allocs(void)
{
    return total_allocs;
}
