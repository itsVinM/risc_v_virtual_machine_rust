#include "uart.hpp"

#include "dtb.hpp"
#include "types.hpp"

namespace kernel::uart {
namespace {

// 8250 UART on the virt platform, MMIO only.
constexpr types::u8 THR = 0;
constexpr types::u8 LSR = 5;
constexpr types::u8 LSR_TX_EMPTY = 1u << 5;

constexpr types::uptr DEFAULT_BASE = 0x10000000UL;

volatile types::u8 *g_base = nullptr;

} // namespace

void init() noexcept
{
    if (!g_base)
        g_base = reinterpret_cast<volatile types::u8 *>(DEFAULT_BASE);
}

void init_from_dtb(const void *dtb) noexcept
{
    types::u64 addr = 0;

    if (dtb) {
        const auto pa =
            static_cast<devicetree::PhysicalAddress>(reinterpret_cast<types::uptr>(dtb));
        if (auto found = devicetree::Dtb::find_uart(pa))
            addr = static_cast<types::u64>(*found);
    }

    g_base = reinterpret_cast<volatile types::u8 *>(addr ? static_cast<types::uptr>(addr)
                                                         : DEFAULT_BASE);
}

void putc(types::u8 c) noexcept
{
    if (!g_base)
        init();

    while (!(g_base[LSR] & LSR_TX_EMPTY))
        ;
    g_base[THR] = c;
}

void puts(const char *s) noexcept
{
    while (*s)
        putc(static_cast<types::u8>(*s++));
}

} // namespace kernel::uart
