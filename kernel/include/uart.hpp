#pragma once

#include "types.hpp"

namespace kernel::uart {
    void init() noexcept;
    // Parses the DTB for /chosen -> stdout-path, falls back to 0x10000000.
    void init_from_dtb(const void *dtb) noexcept;
    void putc(types::u8 c) noexcept;
    void puts(const char *s) noexcept;
} // namespace kernel::uart
