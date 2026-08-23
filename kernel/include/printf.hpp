#pragma once

#include <stdarg.h>

namespace kernel::fmt {
    // Supported conversions: %c %s %d %i %u %x %X %p %b %%
    // Length modifiers: z (size_t), l, ll
    int printf(const char *fmt, ...) noexcept;
    int vprintf(const char *fmt, va_list ap) noexcept;
} // namespace kernel::fmt
