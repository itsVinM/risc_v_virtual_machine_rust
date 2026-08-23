#include "mem.hpp"

#include "printf.hpp"
#include "string.hpp"
#include "types.hpp"

namespace kernel::mem {
namespace {

using types::u8;
using types::usize;

constexpr usize HeapSize = 64u * 1024u;

struct Block {
    usize size;   /* usable bytes (excludes header) */
    int free_;
    Block *next;
    u8 _pad[8];   /* pad to 16-byte boundary */
};

constexpr usize BlockHdrSize = sizeof(Block);
static_assert(BlockHdrSize % 16 == 0);

alignas(16) u8 heap_raw[HeapSize];
Block *heap_list = nullptr;
usize heap_used = 0;   /* tracks payload bytes allocated */
usize heap_allocs = 0;

void heap_init()
{
    auto *list = reinterpret_cast<Block *>(heap_raw);
    list->size = HeapSize - BlockHdrSize;
    list->free_ = 1;
    list->next = nullptr;
    heap_list = list;
    heap_used = 0;
    heap_allocs = 0;
}

} // namespace

void *kmalloc(usize n) noexcept
{
    if (!heap_list)
        heap_init();

    if (n == 0)
        n = 1;
    n = (n + 15) & ~static_cast<usize>(15);

    for (Block *cur = heap_list; cur; cur = cur->next) {
        if (cur->free_ && cur->size >= n) {
            /* split if remainder is large enough for another block + payload */
            if (cur->size >= n + BlockHdrSize + 16) {
                auto *split =
                    reinterpret_cast<Block *>(reinterpret_cast<u8 *>(cur) + BlockHdrSize + n);
                split->size = cur->size - n - BlockHdrSize;
                split->free_ = 1;
                split->next = cur->next;
                cur->next = split;
                cur->size = n;
            }
            cur->free_ = 0;
            heap_used += cur->size;
            heap_allocs++;
            return cur + 1;
        }
    }

    kernel::fmt::printf("kmalloc: out of memory (%zu bytes requested)\n", n);
    return nullptr;
}

void *kzalloc(usize n) noexcept
{
    void *p = kmalloc(n);
    if (p)
        memset(p, 0, n);
    return p;
}

void kfree(void *p) noexcept
{
    if (!p)
        return;

    Block *blk = static_cast<Block *>(p) - 1;
    blk->free_ = 1;
    heap_used -= blk->size;

    /* coalesce with next */
    while (blk->next && blk->next->free_) {
        blk->size += BlockHdrSize + blk->next->size;
        blk->next = blk->next->next;
    }

    /* coalesce with previous */
    Block *prev = nullptr;
    for (Block *cur = heap_list; cur && cur != blk; cur = cur->next)
        prev = cur;

    if (prev && prev->free_) {
        prev->size += BlockHdrSize + blk->size;
        prev->next = blk->next;
    }
}

usize used() noexcept
{
    return heap_used;
}

usize capacity() noexcept
{
    return HeapSize;
}

usize allocs() noexcept
{
    return heap_allocs;
}

} // namespace kernel::mem
