#pragma once

namespace kernel::panic {
    // Prints "PANIC [file:line] msg" over UART and spins forever.
    [[noreturn]] void panic(const char *file, int line, const char *fmt, ...) noexcept;
} // namespace kernel::panic

#define KPANIC(...) ::kernel::panic::panic(__FILE__, __LINE__, __VA_ARGS__)
