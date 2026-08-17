#ifndef TYPES_H
#define TYPES_H

#include <stdint.h>

typedef uint8_t   u8;
typedef uint16_t  u16;
typedef uint32_t  u32;
typedef uint64_t  u64;
typedef u64       usize;
typedef u64       paddr;
typedef u64       vaddr;

#define NULL ((void *)0)

#define PAGE_SIZE 4096UL

#endif
