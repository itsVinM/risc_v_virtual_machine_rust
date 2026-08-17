#include "dtb.h"
#include "string.h"
#include "printf.h"

struct fdt_header {
    u32 magic;
    u32 totalsize;
    u32 off_dt_struct;
    u32 off_dt_strings;
    u32 off_mem_rsvmap;
    u32 version;
    u32 last_comp_version;
    u32 boot_cpuid_phys;
    u32 size_dt_strings;
    u32 size_dt_struct;
};

#define FDT_BEGIN_NODE 0x10000000
#define FDT_END_NODE   0x20000000
#define FDT_PROP       0x30000000
#define FDT_NOP        0x40000000
#define FDT_END        0x90000000

static u32 be32(u32 v)
{
    return ((v & 0xFF) << 24) | ((v & 0xFF00) << 8) |
           ((v >> 8) & 0xFF00) | ((v >> 24) & 0xFF);
}

static u64 align4(u64 v)
{
    return (v + 3) & ~3ULL;
}

/* Parse a property value like "serial@10000000:115200" → 0x10000000 */
static u64 parse_stdout_path(const char *s)
{
    /* skip to '@' */
    while (*s && *s != '@')
        s++;
    if (*s != '@')
        return 0;
    s++;

    /* parse hex digits */
    u64 addr = 0;
    while (*s >= '0' && *s <= '9') {
        addr = addr * 16 + (*s - '0');
        s++;
    }
    while ((*s >= 'a' && *s <= 'f')) {
        addr = addr * 16 + (*s - 'a' + 10);
        s++;
    }
    while ((*s >= 'A' && *s <= 'F')) {
        addr = addr * 16 + (*s - 'A' + 10);
        s++;
    }
    return addr;
}

u64 dtb_find_uart(u64 dtb_pa)
{
    const u8 *base = (const u8 *)dtb_pa;
    const struct fdt_header *hdr = (const struct fdt_header *)base;

    if (be32(hdr->magic) != 0xD00DFEEDUL) {
        printf("dtb: bad magic 0x%x\n", be32(hdr->magic));
        return 0;
    }

    const u8 *structs = base + be32(hdr->off_dt_struct);
    const u8 *strings = base + be32(hdr->off_dt_strings);
    u64 struct_len    = be32(hdr->size_dt_struct);

    int depth = 0;
    u64 pos = 0;
    int in_chosen = 0;
    u64 result = 0;

    while (pos < struct_len) {
        u32 token = be32(*(const u32 *)(structs + pos));
        pos += 4;

        switch (token) {
        case FDT_BEGIN_NODE: {
            /* node name: null-terminated, padded to 4 bytes */
            const char *name = (const char *)(structs + pos);
            u64 name_len = strlen(name) + 1;
            pos = pos + align4(name_len);
            depth++;
            if (depth == 1 && strcmp(name, "chosen") == 0)
                in_chosen = 1;
            break;
        }
        case FDT_END_NODE:
            if (in_chosen && depth == 1)
                in_chosen = 0;
            depth--;
            break;
        case FDT_PROP: {
            u32 len  = be32(*(const u32 *)(structs + pos));
            u32 nameoff = be32(*(const u32 *)(structs + pos + 4));
            const char *prop_name = (const char *)(strings + nameoff);
            const u8 *val = structs + pos + 8;
            pos = pos + 8 + align4(len);

            if (in_chosen && strcmp(prop_name, "stdout-path") == 0) {
                result = parse_stdout_path((const char *)val);
                printf("dtb: stdout-path = \"%s\" → uart=0x%lx\n", val, result);
            }
            break;
        }
        case FDT_NOP:
            break;
        case FDT_END:
            return result;
        default:
            printf("dtb: unknown token 0x%x at offset %lu\n", token, pos - 4);
            return result;
        }
    }

    return result;
}
