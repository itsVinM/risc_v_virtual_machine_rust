#include "printf.hpp"

#include <stdarg.h>
#include <stdint.h>

#include "types.hpp"
#include "uart.hpp"

namespace kernel::fmt {
namespace {

using types::u64;

void putc(char c)
{
    uart::putc(static_cast<types::u8>(c));
}

void print_u64(u64 v, unsigned base, bool upper, int minw, char pad)
{
    char buf[65];
    int i = 0;

    if (v == 0)
        buf[i++] = '0';
    while (v) {
        const u64 d = v % base;
        v /= base;
        buf[i++] = static_cast<char>(d < 10 ? '0' + d : (upper ? 'A' : 'a') + (d - 10));
    }

    int n = i;
    while (n < minw) {
        putc(pad);
        ++n;
    }
    while (i)
        putc(buf[--i]);
}

void print_int(long v, int minw, char pad)
{
    if (v < 0) {
        putc('-');
        v = -v;
    }
    print_u64(static_cast<u64>(v), 10, false, minw, pad);
}

} // namespace

int vprintf(const char *fmt, va_list ap) noexcept
{
    int count = 0;

    for (; *fmt; fmt++) {
        if (*fmt != '%') {
            putc(*fmt);
            count++;
            continue;
        }
        fmt++;

        int minw = 0;
        char pad = ' ';
        if (*fmt == '0') {
            pad = '0';
            fmt++;
        }
        while (*fmt >= '0' && *fmt <= '9') {
            minw = minw * 10 + (*fmt - '0');
            fmt++;
        }

        int is_long = 0;
        bool is_size_t = false;
        if (*fmt == 'z') {
            is_size_t = true;
            fmt++;
        } else if (*fmt == 'l') {
            is_long = 1;
            fmt++;
            if (*fmt == 'l') {
                is_long = 2;
                fmt++;
            }
        }

        switch (*fmt) {
        case 'c':
            putc(static_cast<char>(va_arg(ap, int)));
            count++;
            break;
        case 's': {
            const char *s = va_arg(ap, const char *);
            if (!s)
                s = "(null)";
            while (*s) {
                putc(*s++);
                count++;
            }
            break;
        }
        case 'd':
        case 'i':
            if (is_long)
                print_int(va_arg(ap, long), minw, pad);
            else
                print_int(va_arg(ap, int), minw, pad);
            count++;
            break;
        case 'u':
            if (is_size_t)
                print_u64(va_arg(ap, types::usize), 10, false, minw, pad);
            else if (is_long)
                print_u64(va_arg(ap, unsigned long), 10, false, minw, pad);
            else
                print_u64(va_arg(ap, unsigned), 10, false, minw, pad);
            count++;
            break;
        case 'x':
            if (is_long)
                print_u64(va_arg(ap, unsigned long), 16, false, minw, pad);
            else
                print_u64(va_arg(ap, unsigned), 16, false, minw, pad);
            count++;
            break;
        case 'X':
            if (is_long)
                print_u64(va_arg(ap, unsigned long), 16, true, minw, pad);
            else
                print_u64(va_arg(ap, unsigned), 16, true, minw, pad);
            count++;
            break;
        case 'p':
            print_u64(reinterpret_cast<u64>(va_arg(ap, void *)), 16, false, 0, ' ');
            count++;
            break;
        case 'b':
            if (is_long)
                print_u64(va_arg(ap, unsigned long), 2, false, minw, pad);
            else
                print_u64(va_arg(ap, unsigned), 2, false, minw, pad);
            count++;
            break;
        case '%':
            putc('%');
            count++;
            break;
        default:
            putc('%');
            putc(*fmt);
            count += 2;
            break;
        }
    }

    return count;
}

int printf(const char *fmt, ...) noexcept
{
    va_list ap;
    va_start(ap, fmt);
    const int ret = vprintf(fmt, ap);
    va_end(ap);
    return ret;
}

} // namespace kernel::fmt
