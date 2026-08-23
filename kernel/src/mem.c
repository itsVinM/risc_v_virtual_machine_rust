#include "mem.h"
#include "types.h"
#include "string.h"
#include "printf.h"
#include <stdint.h>

#define HEAP_SIZE (64u * 1024u)

typedef struct block {
    size_t size;       /* usable bytes (excludes header) */
    int    free;       /* 1 = free, 0 = allocated */
    struct block *next;
    u8 _pad[8];        /* pad to 16-byte boundary */
} block_t;

#define BLOCK_HDR_SIZE sizeof(block_t)

static u8 heap_raw[HEAP_SIZE] __attribute__((aligned(16)));
static block_t *heap_list;
static size_t  heap_used;     /* tracks payload bytes allocated */
static size_t  heap_allocs;

static void heap_init(void)
{
    heap_list = (block_t *)heap_raw;
    heap_list->size = HEAP_SIZE - BLOCK_HDR_SIZE;
    heap_list->free = 1;
    heap_list->next = 0;
    heap_used = 0;
    heap_allocs = 0;
}

void *kmalloc(size_t n)
{
    block_t *cur;

    if (!heap_list)
        heap_init();

    if (n == 0)
        n = 1;
    n = (n + 15) & ~(size_t)15;

    cur = heap_list;
    while (cur) {
        if (cur->free && cur->size >= n) {
            /* split if remainder is large enough for another block + payload */
            if (cur->size >= n + BLOCK_HDR_SIZE + 16) {
                block_t *split = (block_t *)((u8 *)cur + BLOCK_HDR_SIZE + n);
                split->size = cur->size - n - BLOCK_HDR_SIZE;
                split->free = 1;
                split->next = cur->next;
                cur->next = split;
                cur->size = n;
            }
            cur->free = 0;
            heap_used += cur->size;
            heap_allocs++;
            return (void *)(cur + 1);
        }
        cur = cur->next;
    }

    printf("kmalloc: out of memory (%zu bytes requested)\n", n);
    return 0;
}

void *kzalloc(size_t n)
{
    void *p = kmalloc(n);
    if (p)
        memset(p, 0, n);
    return p;
}

void kfree(void *p)
{
    block_t *blk, *prev, *cur;

    if (!p)
        return;

    blk = (block_t *)p - 1;
    blk->free = 1;
    heap_used -= blk->size;

    /* coalesce with next */
    while (blk->next && blk->next->free) {
        blk->size += BLOCK_HDR_SIZE + blk->next->size;
        blk->next = blk->next->next;
    }

    /* coalesce with previous */
    prev = 0;
    cur = heap_list;
    while (cur && cur != blk) {
        prev = cur;
        cur = cur->next;
    }
    if (prev && prev->free) {
        prev->size += BLOCK_HDR_SIZE + blk->size;
        prev->next = blk->next;
    }
}

size_t kmem_used(void)
{
    return heap_used;
}

size_t kmem_capacity(void)
{
    return HEAP_SIZE;
}

size_t kmem_allocs(void)
{
    return heap_allocs;
}
