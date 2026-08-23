#include "uart.h"
#include <stdint.h>

/* 8250 UART on the virt platform, MMIO only. */
#define UART_THR 0
#define UART_LSR 5
#define UART_LSR_TX_EMPTY (1u << 5)

static volatile uint8_t *uart;
static int uart_inited;

void uart_init(void)
{
    if (!uart)
        uart = (volatile uint8_t *)0x10000000UL;
    uart_inited = 1;
}

/* --- minimal DTB parser -------------------------------------------------- */

#define FDT_MAGIC 0xd00dfeed

struct fdt_header {
    uint32_t magic;
    uint32_t totalsize;
    uint32_t off_struct;
    uint32_t off_strings;
    uint32_t off_mem_rsvmap;
    uint32_t version;
    uint32_t last_comp_version;
    uint32_t boot_cpuid_phys;
    uint32_t size_dt_strings;
    uint32_t size_dt_struct;
};

static uint32_t be32(const void *p)
{
    const uint8_t *b = p;
    return ((uint32_t)b[0] << 24) | ((uint32_t)b[1] << 16) |
           ((uint32_t)b[2] << 8)  |  (uint32_t)b[3];
}

static uint64_t be64(const void *p)
{
    return ((uint64_t)be32(p) << 32) | (uint64_t)be32((const uint8_t *)p + 4);
}

static int streq(const char *a, const char *b)
{
    while (*a && *b) {
        if (*a++ != *b++)
            return 0;
    }
    return *a == *b;
}

static int strlen_s(const char *s)
{
    int n = 0;
    while (*s++)
        n++;
    return n;
}

/* Return pointer to the last path component: "/soc/serial@10000000" -> "serial@10000000" */
static const char *path_leaf(const char *path)
{
    const char *leaf = path;
    while (*path) {
        if (*path == '/')
            leaf = path + 1;
        path++;
    }
    return leaf;
}

/*
 * Parse the DTB to find the UART base address.
 * 1. Walk the struct block to find /chosen -> stdout-path
 * 2. Extract the leaf node name from stdout-path
 * 3. Walk again to find that node's "reg" property
 * Returns 0 on failure (caller falls back to hardcoded address).
 */
static uint64_t dtb_find_uart(const void *dtb)
{
    const uint8_t *base = dtb;
    const struct fdt_header *hdr = dtb;
    uint32_t struct_off, strings_off;
    const uint8_t *structs, *strings, *p;
    char target[128];
    uint64_t addr;

    if (be32(&hdr->magic) != FDT_MAGIC)
        return 0;

    struct_off  = be32(&hdr->off_struct);
    strings_off = be32(&hdr->off_strings);
    structs = base + struct_off;
    strings = base + strings_off;

    /* --- pass 1: find stdout-path in /chosen --- */
    target[0] = '\0';
    {
        int depth = 0, in_chosen = 0;

        p = structs;
        for (;;) {
            uint32_t tok = be32(p); p += 4;
            if (tok == 0x09) break;           /* FDT_END */

            if (tok == 0x01) {                /* FDT_BEGIN_NODE */
                const char *name = (const char *)p;
                int len = strlen_s(name) + 1;
                p = (const uint8_t *)(((uintptr_t)p + len + 3) & ~3u);
                if (depth == 0 && streq(name, "chosen"))
                    in_chosen = 1;
                depth++;
            } else if (tok == 0x02) {         /* FDT_END_NODE */
                if (depth == 1) in_chosen = 0;
                depth--;
            } else if (tok == 0x03) {         /* FDT_PROP */
                uint32_t len     = be32(p); p += 4;
                uint32_t nameoff = be32(p); p += 4;
                const char *pname = (const char *)(strings + nameoff);
                if (in_chosen && streq(pname, "stdout-path")) {
                    int n = len < 127 ? len : 127;
                    int i;
                    for (i = 0; i < n; i++)
                        target[i] = ((const char *)p)[i];
                    target[i] = '\0';
                }
                p = (const uint8_t *)(((uintptr_t)p + len + 3) & ~3u);
            } else if (tok == 0x04) {         /* FDT_NOP */
                /* skip */
            }
        }
    }

    if (target[0] == '\0')
        return 0;

    /* strip address cell after ':' if present, e.g. "serial@10000000:115200" */
    {
        int i = 0;
        while (target[i] && target[i] != ':')
            i++;
        target[i] = '\0';
    }

    {
        const char *leaf = path_leaf(target);
        int depth = 0, found = 0;

        /* pass 2: find that node, read its "reg" property */
        addr = 0;
        p = structs;
        for (;;) {
            uint32_t tok = be32(p); p += 4;
            if (tok == 0x09) break;

            if (tok == 0x01) {
                const char *name = (const char *)p;
                int len = strlen_s(name) + 1;
                p = (const uint8_t *)(((uintptr_t)p + len + 3) & ~3u);
                if (depth == 1 && streq(name, leaf))
                    found = 1;
                depth++;
            } else if (tok == 0x02) {
                if (depth == 1) found = 0;
                depth--;
            } else if (tok == 0x03) {
                uint32_t len     = be32(p); p += 4;
                uint32_t nameoff = be32(p); p += 4;
                const char *pname = (const char *)(strings + nameoff);
                if (found && streq(pname, "reg")) {
                    /* #address-cells=2, #size-cells=2 on root */
                    if (len >= 8)
                        addr = be64(p);
                    else if (len >= 4)
                        addr = (uint64_t)be32(p);
                }
                p = (const uint8_t *)(((uintptr_t)p + len + 3) & ~3u);
            } else if (tok == 0x04) {
                /* FDT_NOP */
            }
        }
    }

    return addr;
}

void uart_init_from_dtb(const void *dtb)
{
    uint64_t addr = 0;
    if (dtb)
        addr = dtb_find_uart(dtb);
    if (addr)
        uart = (volatile uint8_t *)addr;
    else
        uart = (volatile uint8_t *)0x10000000UL;
    uart_inited = 1;
}

void uart_putc(uint8_t c)
{
    while (!(uart[UART_LSR] & UART_LSR_TX_EMPTY))
        ;
    uart[UART_THR] = c;
}

void uart_puts(const char *s)
{
    while (*s)
        uart_putc((uint8_t)*s++);
}
