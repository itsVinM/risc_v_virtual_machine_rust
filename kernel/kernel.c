#include <stdint.h>
#include "uart.h"
#include "printf.h"
#include "mem.h"

typedef uint64_t u64;
typedef uint32_t u32;
typedef uint8_t u8;

void kmain(u64 hartid, u64 dtb);

static void self_tests(void);

/*
 * Reset entry point. Pure C: the stack pointer is provided by the boot
 * platform before entry (VM: sp = DRAM_END at reset; QEMU: OpenSBI sets sp).
 * a0/a1 carry hartid + DTB address, matching the S-mode boot protocol.
 */
__attribute__((section(".text.entry")))
void _start(u64 hartid, u64 dtb)
{
    kmain(hartid, dtb);
}

void kmain(u64 hartid, u64 dtb)
{
    uart_init();

    printf("rv64vm C kernel: boot complete\n");
    printf("  hartid=%lu  dtb=0x%p\n", hartid, (void *)dtb);
    printf("  kernel @ 0x%p  dram 0x%p..0x%p\n",
           (void *)0x80200000UL, (void *)0x80000000UL, (void *)0x88000000UL);

    self_tests();

    printf("kernel: tests done - spinning forever (no SBI)\n");
    for (;;) {
        volatile u64 spins = 0;
        spins++;
    }
}

static void self_tests(void)
{
    u64 v = 0xdeadbeefULL;
    u32 *p;
    unsigned i;
    void *a, *b, *c;

    printf("[self-test] arithmetic: 3*7=%d  84/5=%d  84%%5=%d  1<<24=%lu\n",
           3 * 7, 84 / 5, 84 % 5, (unsigned long)(1u << 24));
    printf("[self-test] bitops: 0xdead^0xbeef=0x%x  ~0u=0x%lx  v&0xf=0x%x\n",
           (unsigned)(0xdeadu ^ 0xbeefu), ~0ul, (unsigned)(v & 0xf));

    a = kzalloc(32);
    b = kmalloc(1024);
    c = kmalloc(4096);
    printf("[self-test] kmalloc: a=0x%p b=0x%p c=0x%p\n", a, b, c);
    printf("[self-test] heap: %lu/%lu bytes used across %lu allocs\n",
           kmem_used(), kmem_capacity(), kmem_allocs());

    if ((((u64)(uintptr_t)a | (u64)(uintptr_t)b | (u64)(uintptr_t)c) & 15u) == 0)
        printf("[self-test] alignment: ok\n");
    else
        printf("[self-test] alignment: FAIL\n");

    p = kmalloc(64);
    for (i = 0; i < 16; i++)
        p[i] = (u32)((u64)i * i * 2654435761u);
    for (i = 0; i < 16; i++) {
        if (p[i] != (u32)((u64)i * i * 2654435761u)) {
            printf("[self-test] dram readback: FAIL at %u\n", i);
            return;
        }
    }
    printf("[self-test] dram readback: ok\n");
}
