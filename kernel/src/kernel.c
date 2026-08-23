#include <stdint.h>
#include <stdarg.h>
#include "types.h"
#include "uart.h"
#include "printf.h"
#include "mem.h"
#include "string.h"
#include "sbi.h"

extern char _bss_start[], _bss_end[];
extern uint64_t _stack_canary[];

#define STACK_CANARY 0xDEADBEEFCAFEBABEULL

void kmain(u64 hartid, u64 dtb);
static void self_tests(void);

/*
 * Minimal Sv39 identity-map: two gigapages.
 *   PML4[0] -> 0x00000000 (covers UART at 0x10000000, CLINT, PLIC)
 *   PML4[2] -> 0x80000000 (covers kernel code/data/stack)
 */
static uint64_t root_pml4[512] __attribute__((aligned(4096)));

static void setup_vm(void)
{
    /* V|R|W|X|G|A|D = 0xEF */
    root_pml4[0] = ((0x00000000ULL >> 12) << 10) | 0xEF;  /* gigapage @ 0x0 */
    root_pml4[2] = ((0x80000000ULL >> 12) << 10) | 0xEF;  /* gigapage @ 0x80000000 */

    uint64_t satp_val = (8ULL << 60) | ((uint64_t)root_pml4 >> 12);
    asm volatile(
        "csrw satp, %0\n"
        "sfence.vma zero, zero\n"
        :: "r"(satp_val)
        : "memory"
    );
}

/*
 * Reset entry point.  The VM boots us in M-mode at 0x80200000.
 * a0 = hartid, a1 = dtb physical address.
 */
__attribute__((section(".text.entry")))
void _start(u64 hartid, u64 dtb)
{
    /* 1. Zero BSS */
    {
        char *p = _bss_start;
        char *end = _bss_end;
        while (p < end)
            *p++ = 0;
    }

    /* 2. Write stack canary */
    _stack_canary[0] = STACK_CANARY;

    /* 3. Install M-mode trap handler */
    extern void trap_handler(void);
    asm volatile("csrw mtvec, %0" :: "r"(&trap_handler));

    kmain(hartid, dtb);
}

void kmain(u64 hartid, u64 dtb)
{
    /* Check stack canary */
    if (_stack_canary[0] != STACK_CANARY)
        panic("kernel", __LINE__, "stack smashed");

    /* UART from DTB (falls back to 0x10000000 on parse failure) */
    uart_init_from_dtb((const void *)dtb);

    /* Enable Sv39 identity mapping */
    setup_vm();

    printf("rv64vm C kernel: boot complete\n");
    printf("  hartid=%lu  dtb=0x%p\n", hartid, (void *)dtb);
    printf("  kernel @ 0x%p  dram 0x%p..0x%p\n",
           (void *)0x80200000UL, (void *)0x80000000UL, (void *)0x88000000UL);

    self_tests();

    printf("kernel: all tests passed\n");
    printf("kernel: spinning forever (no SBI scheduler)\n");
    for (;;)
        asm volatile("wfi");
}

static void self_tests(void)
{
    u64 v = 0xdeadbeefULL;
    u32 *p;
    unsigned i;
    void *a, *b, *c, *d;

    printf("[self-test] arithmetic: 3*7=%d  84/5=%d  84%%5=%d  1<<24=%lu\n",
           3 * 7, 84 / 5, 84 % 5, (unsigned long)(1u << 24));
    printf("[self-test] bitops: 0xdead^0xbeef=0x%x  ~0u=0x%lx  v&0xf=0x%x\n",
           (unsigned)(0xdeadu ^ 0xbeefu), ~0ul, (unsigned)(v & 0xf));

    a = kzalloc(32);
    b = kmalloc(1024);
    c = kmalloc(4096);
    printf("[self-test] kmalloc: a=0x%p b=0x%p c=0x%p\n", a, b, c);
    printf("[self-test] heap: %zu/%zu bytes used across %zu allocs\n",
           kmem_used(), kmem_capacity(), kmem_allocs());

    if ((((u64)(uintptr_t)a | (u64)(uintptr_t)b | (u64)(uintptr_t)c) & 15u) != 0)
        panic("kernel", __LINE__, "alignment check failed");
    printf("[self-test] alignment: ok\n");

    p = kmalloc(64);
    for (i = 0; i < 16; i++)
        p[i] = (u32)((u64)i * i * 2654435761u);
    for (i = 0; i < 16; i++) {
        if (p[i] != (u32)((u64)i * i * 2654435761u))
            panic("kernel", __LINE__, "dram readback failed at %u", i);
    }
    printf("[self-test] dram readback: ok\n");

    /* test kfree + coalescing */
    kfree(b);
    d = kmalloc(512);
    printf("[self-test] kfree: freed b=%p, new d=%p\n", b, d);

    printf("[self-test] memset: ");
    {
        u8 *m = kmalloc(128);
        memset(m, 0xAB, 128);
        for (i = 0; i < 128; i++) {
            if (m[i] != 0xAB)
                panic("kernel", __LINE__, "memset failed at byte %u", i);
        }
        printf("ok");
    }
    printf("\n[self-test] memcpy: ");
    {
        u8 src[64], dst[64];
        for (i = 0; i < 64; i++) src[i] = (u8)(i * 3 + 7);
        memcpy(dst, src, 64);
        for (i = 0; i < 64; i++) {
            if (dst[i] != src[i])
                panic("kernel", __LINE__, "memcpy failed at byte %u", i);
        }
        printf("ok");
    }
    printf("\n");
}
