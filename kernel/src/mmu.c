#include "mmu.h"
#include "mem.h"
#include "string.h"
#include "csr.h"
#include "printf.h"

/* Sv39 constants */
#define SATP_MODE_SV39     (8UL << 60)
#define PTE_V              (1UL << 0)
#define PTE_R              (1UL << 1)
#define PTE_W              (1UL << 2)
#define PTE_X              (1UL << 3)
#define PTE_A              (1UL << 6)
#define PTE_D              (1UL << 7)
#define PTE_PPN_SHIFT      10

#define VA_VPN2(va) (((u64)(va) >> 30) & 0x1FF)
#define VA_VPN1(va) (((u64)(va) >> 20) & 0x1FF)
#define VA_VPN0(va) (((u64)(va) >> 12) & 0x1FF)

/* Page frame allocator — bump allocator, page-aligned, never freed.
 * Page table pages are allocated once at boot and live forever. */
static usize pf_used = 0;

static void *pf_alloc(void)
{
    extern unsigned char _end[];
    paddr base = ((paddr)_end + PAGE_SIZE - 1) & ~(PAGE_SIZE - 1);
    void *p = (void *)(base + pf_used);
    pf_used += PAGE_SIZE;
    memset(p, 0, PAGE_SIZE);
    return p;
}

static void map_page(u64 *root, vaddr va, paddr pa, u64 perm)
{
    u64 *l1, *l0;

    if (!(root[VA_VPN2(va)] & PTE_V)) {
        l1 = pf_alloc();
        root[VA_VPN2(va)] = ((u64)l1 >> 2) | PTE_V;
    } else {
        l1 = (u64 *)((root[VA_VPN2(va)] >> 2) << 12);
    }

    if (!(l1[VA_VPN1(va)] & PTE_V)) {
        l0 = pf_alloc();
        l1[VA_VPN1(va)] = ((u64)l0 >> 2) | PTE_V;
    } else {
        l0 = (u64 *)((l1[VA_VPN1(va)] >> 2) << 12);
    }

    l0[VA_VPN0(va)] = (pa >> 2) | perm | PTE_V | PTE_A | PTE_D;
}

static void map_range(u64 *root, paddr base, u64 len, u64 perm)
{
    for (u64 off = 0; off < len; off += PAGE_SIZE)
        map_page(root, base + off, base + off, perm);
}

static u64 *root_table = NULL;

u64 *mmu_root(void)
{
    return root_table;
}

void mmu_init(void)
{
    extern unsigned char _start[];
    extern unsigned char _end[];

    root_table = pf_alloc();

    paddr kernel_start = (paddr)_start;
    paddr kernel_end   = ((paddr)_end + PAGE_SIZE - 1) & ~(PAGE_SIZE - 1);
    u64 kernel_size    = kernel_end - kernel_start;

    /* identity-map the kernel (phys == virt) so we don't explode on sret */
    map_range(root_table, kernel_start, kernel_size, PTE_R | PTE_X);

    /* identity-map UART so printf keeps working */
    map_range(root_table, 0x10000000UL, PAGE_SIZE, PTE_R | PTE_W);

    /* identity-map the page frame allocator region so pf_alloc keeps working */
    map_range(root_table, (paddr)root_table, PAGE_SIZE, PTE_R | PTE_W);

    u64 satp = SATP_MODE_SV39 | ((u64)root_table >> 12);
    CSR_WRITE(satp, satp);
    asm volatile("sfence.vma" ::: "memory");

    printf("mmu: Sv39 enabled, satp=0x%lx root=0x%lx\n", satp, (u64)root_table);
    printf("mmu: mapped kernel 0x%lx..0x%lx (%lu pages)\n",
           kernel_start, kernel_end, kernel_size / PAGE_SIZE);
}
