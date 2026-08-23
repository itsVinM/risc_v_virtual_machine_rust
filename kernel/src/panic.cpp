#include "panic.hpp"

#include <stdarg.h>

#include "printf.hpp"
#include "uart.hpp"

namespace kernel::panic {

void panic(const char *file, int line, const char *fmt, ...) noexcept
{
    va_list ap;

    uart::puts("PANIC [");
    uart::puts(file);
    uart::putc(':');
    kernel::fmt::printf("%d", line);
    uart::puts("] ");

    va_start(ap, fmt);
    (void)kernel::fmt::vprintf(fmt, ap);
    va_end(ap);

    uart::putc('\n');

    for (;;)
        asm volatile("wfi");
}

} // namespace kernel::panic
