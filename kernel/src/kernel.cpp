#include <cstdint>

#include "csr.hpp"
#include "mem.hpp"
#include "mmu.hpp"
#include "panic.hpp"
#include "printf.hpp"
#include "sbi.hpp"
#include "string.hpp"
#include "trap.hpp"
#include "types.hpp"
#include "uart.hpp"

extern "C" char _bss_start[];
extern "C" char _bss_end[];
extern "C" kernel::types::u64 _stack_canary[];

#define STACK_CANARY 0xDEADBEEFCAFEBABEULL

void kmain(kernel::types::u64 hartid, kernel::types::u64 dtb);
static void self_tests();

/*
 * Reset entry point. The VM boots us in M-mode at 0x80200000.
 * a0 = hartid, a1 = dtb physical address.
 */
extern "C" __attribute__((section(".text.entry")))
void _start(kernel::types::u64 hartid, kernel::types::u64 dtb)
{
    /* 1. Zero BSS before anything touches globals */
    for (char *p = _bss_start; p < _bss_end; ++p)
        *p = 0;

    /* 2. Write stack canary */
    _stack_canary[0] = STACK_CANARY;

    /* 3. Install M-mode trap handler */
    kernel::arch::rv64vm::csr_write<kernel::arch::rv64vm::Csr::Mtvec>(
        reinterpret_cast<kernel::types::uptr>(&kernel::trap::trap_handler));

    kmain(hartid, dtb);
}

void kmain(kernel::types::u64 hartid, kernel::types::u64 dtb)
{
    /* Check stack canary */
    if (_stack_canary[0] != STACK_CANARY)
        kernel::panic::panic("kernel", __LINE__, "stack smashed");

    /* UART from DTB (falls back to 0x10000000 on parse failure) */
    kernel::uart::init_from_dtb(reinterpret_cast<const void *>(dtb));

    /* Enable Sv39 identity mapping */
    kernel::rv64vm::Mmu::init();

    kernel::fmt::printf("rv64vm C++20 kernel: boot complete\n");
    kernel::fmt::printf("  hartid=%lu  dtb=0x%p\n", hartid,
                        reinterpret_cast<void *>(static_cast<kernel::types::uptr>(dtb)));
    kernel::fmt::printf("  kernel @ 0x%p  dram 0x%p..0x%p\n",
                        reinterpret_cast<void *>(0x80200000UL),
                        reinterpret_cast<void *>(0x80000000UL),
                        reinterpret_cast<void *>(0x88000000UL));

    self_tests();

    kernel::fmt::printf("kernel: all tests passed\n");
    kernel::fmt::printf("kernel: spinning forever (no SBI scheduler)\n");
    for (;;)
        asm volatile("wfi");
}

static void self_tests()
{
    namespace kmem = kernel::mem;
    using kernel::types::u32;
    using kernel::types::u64;
    using kernel::types::u8;

    const u64 v = 0xdeadbeefULL;

    kernel::fmt::printf("[self-test] arithmetic: 3*7=%d  84/5=%d  84%%5=%d  1<<24=%lu\n",
                        3 * 7, 84 / 5, 84 % 5, static_cast<unsigned long>(1u << 24));
    kernel::fmt::printf("[self-test] bitops: 0xdead^0xbeef=0x%x  ~0u=0x%lx  v&0xf=0x%x\n",
                        static_cast<unsigned>(0xdeadu ^ 0xbeefu), ~0ul,
                        static_cast<unsigned>(v & 0xf));

    void *a = kmem::kzalloc(32);
    void *b = kmem::kmalloc(1024);
    void *c = kmem::kmalloc(4096);
    kernel::fmt::printf("[self-test] kmalloc: a=0x%p b=0x%p c=0x%p\n", a, b, c);
    kernel::fmt::printf("[self-test] heap: %zu/%zu bytes used across %zu allocs\n",
                        kmem::used(), kmem::capacity(), kmem::allocs());

    const auto bits =
        reinterpret_cast<kernel::types::uptr>(a) | reinterpret_cast<kernel::types::uptr>(b) |
        reinterpret_cast<kernel::types::uptr>(c);
    if ((bits & 15u) != 0)
        kernel::panic::panic("kernel", __LINE__, "alignment check failed");
    kernel::fmt::printf("[self-test] alignment: ok\n");

    u32 *p = static_cast<u32 *>(kmem::kmalloc(64));
    for (kernel::types::usize i = 0; i < 16; i++)
        p[i] = static_cast<u32>(static_cast<u64>(i) * i * 2654435761ull);
    for (kernel::types::usize i = 0; i < 16; i++) {
        if (p[i] != static_cast<u32>(static_cast<u64>(i) * i * 2654435761ull))
            kernel::panic::panic("kernel", __LINE__, "dram readback failed at %u",
                                 static_cast<unsigned>(i));
    }
    kernel::fmt::printf("[self-test] dram readback: ok\n");

    /* test kfree + coalescing */
    kmem::kfree(b);
    void *d = kmem::kmalloc(512);
    kernel::fmt::printf("[self-test] kfree: freed b=%p, new d=%p\n", b, d);

    kernel::fmt::printf("[self-test] memset: ");
    {
        u8 *m = static_cast<u8 *>(kmem::kmalloc(128));
        memset(m, 0xAB, 128);
        for (kernel::types::usize i = 0; i < 128; i++) {
            if (m[i] != 0xAB)
                kernel::panic::panic("kernel", __LINE__, "memset failed at byte %u",
                                     static_cast<unsigned>(i));
        }
        kernel::fmt::printf("ok");
    }
    kernel::fmt::printf("\n[self-test] memcpy: ");
    {
        u8 src[64], dst[64];
        for (kernel::types::usize i = 0; i < 64; i++)
            src[i] = static_cast<u8>(i * 3 + 7);
        memcpy(dst, src, 64);
        for (kernel::types::usize i = 0; i < 64; i++) {
            if (dst[i] != src[i])
                kernel::panic::panic("kernel", __LINE__, "memcpy failed at byte %u",
                                     static_cast<unsigned>(i));
        }
        kernel::fmt::printf("ok");
    }
    kernel::fmt::printf("\n");
}
