#include "dtb.hpp"

#include "types.hpp"

namespace kernel::devicetree {
namespace {

using types::u32;
using types::u64;
using types::u8;
using types::uptr;
using types::usize;

constexpr u32 FDT_MAGIC      = 0xd00dfeed;
constexpr u32 FDT_BEGIN_NODE = 0x1;
constexpr u32 FDT_END_NODE   = 0x2;
constexpr u32 FDT_PROP       = 0x3;
constexpr u32 FDT_NOP        = 0x4;
constexpr u32 FDT_END        = 0x9;

struct FdtHeader {
    u32 magic;
    u32 totalsize;
    u32 off_struct;
    u32 off_strings;
    u32 off_mem_rsvmap;
    u32 version;
    u32 last_comp_version;
    u32 boot_cpuid_phys;
    u32 size_dt_strings;
    u32 size_dt_struct;
};

[[nodiscard]] u32 be32(const void *p)
{
    const auto *b = static_cast<const u8 *>(p);
    return (static_cast<u32>(b[0]) << 24) | (static_cast<u32>(b[1]) << 16) |
           (static_cast<u32>(b[2]) << 8) | static_cast<u32>(b[3]);
}

[[nodiscard]] u64 be64(const void *p)
{
    const auto *b = static_cast<const u8 *>(p);
    return (static_cast<u64>(be32(b)) << 32) | be32(b + 4);
}

// Advance past a length-padded blob, keeping 4-byte alignment.
[[nodiscard]] const u8 *advance(const u8 *p, usize len)
{
    return reinterpret_cast<const u8 *>((reinterpret_cast<uptr>(p) + len + 3) & ~uptr{3});
}

[[nodiscard]] bool streq(const char *a, const char *b)
{
    while (*a && *b) {
        if (*a++ != *b++)
            return false;
    }
    return *a == *b;
}

[[nodiscard]] usize str_len(const char *s)
{
    usize n = 0;
    while (*s++)
        ++n;
    return n;
}

// "/soc/serial@10000000" -> "serial@10000000"
[[nodiscard]] const char *path_leaf(const char *path)
{
    const char *leaf = path;
    for (const char *s = path; *s; ++s) {
        if (*s == '/')
            leaf = s + 1;
    }
    return leaf;
}

} // namespace

std::optional<PhysicalAddress> Dtb::find_uart(PhysicalAddress dtb_pa) noexcept
{
    const auto *base = reinterpret_cast<const u8 *>(static_cast<u64>(dtb_pa));
    const auto *hdr = reinterpret_cast<const FdtHeader *>(base);

    if (be32(&hdr->magic) != FDT_MAGIC)
        return std::nullopt;

    const u8 *structs = base + be32(&hdr->off_struct);
    const u8 *strings = base + be32(&hdr->off_strings);

    /* --- pass 1: read /chosen -> stdout-path ------------------------------ */
    char target[128] = {};
    {
        int depth = 0;
        int chosen_depth = -1;
        const u8 *p = structs;

        for (;;) {
            const u32 tok = be32(p);
            p += 4;

            if (tok == FDT_END)
                break;

            switch (tok) {
            case FDT_BEGIN_NODE: {
                const char *name = reinterpret_cast<const char *>(p);
                p = advance(p, str_len(name) + 1);
                ++depth;
                if (chosen_depth < 0 && streq(name, "chosen"))
                    chosen_depth = depth;
                break;
            }
            case FDT_END_NODE:
                if (depth == chosen_depth)
                    chosen_depth = -1;
                --depth;
                break;
            case FDT_PROP: {
                const u32 len = be32(p);
                const u32 nameoff = be32(p + 4);
                const char *pname = reinterpret_cast<const char *>(strings + nameoff);
                const u8 *val = p + 8;
                p = advance(p, 8 + len);

                if (depth == chosen_depth && streq(pname, "stdout-path")) {
                    const usize n = len < 127 ? len : 127;
                    for (usize i = 0; i < n; i++)
                        target[i] = static_cast<char>(val[i]);
                    target[n] = '\0';
                }
                break;
            }
            case FDT_NOP:
                break;
            default:
                return std::nullopt; // malformed struct block
            }
        }
    }

    if (target[0] == '\0')
        return std::nullopt;

    /* strip ":115200"-style options suffix */
    for (char *t = target; *t; ++t) {
        if (*t == ':') {
            *t = '\0';
            break;
        }
    }

    const char *leaf = path_leaf(target);

    /* --- pass 2: find the node, read its "reg" property -------------------- */
    u64 addr = 0;
    {
        int depth = 0;
        int found_depth = -1;
        const u8 *p = structs;

        for (;;) {
            const u32 tok = be32(p);
            p += 4;

            if (tok == FDT_END)
                break;

            switch (tok) {
            case FDT_BEGIN_NODE: {
                const char *name = reinterpret_cast<const char *>(p);
                p = advance(p, str_len(name) + 1);
                ++depth;
                if (found_depth < 0 && streq(name, leaf))
                    found_depth = depth;
                break;
            }
            case FDT_END_NODE:
                if (depth == found_depth)
                    found_depth = -1;
                --depth;
                break;
            case FDT_PROP: {
                const u32 len = be32(p);
                const u32 nameoff = be32(p + 4);
                const char *pname = reinterpret_cast<const char *>(strings + nameoff);
                const u8 *val = p + 8;
                p = advance(p, 8 + len);

                if (depth == found_depth && streq(pname, "reg")) {
                    /* #address-cells=2, #size-cells=2 on the virt root */
                    if (len >= 8)
                        addr = be64(val);
                    else if (len >= 4)
                        addr = be32(val);
                }
                break;
            }
            case FDT_NOP:
                break;
            default:
                return addr != 0 ? std::optional<PhysicalAddress>(static_cast<PhysicalAddress>(addr))
                                 : std::nullopt;
            }
        }
    }

    if (addr == 0)
        return std::nullopt;
    return std::optional<PhysicalAddress>(static_cast<PhysicalAddress>(addr));
}

} // namespace kernel::devicetree
