#include <stdarg.h>
#include "printf.h"
#include "uart.h"
#include "types.h"

static void putc(char c)
{
    uart_putc((u8)c);
}

/* buf must hold 65 chars: 64 binary digits + null */
static void print_u64(u64 v, unsigned base, int upper, int minw, char pad)
{
    char buf[65];
    int i = 0;
    if (v == 0)
        buf[i++] = '0';
    else {
        while (v) {
            u64 d = v % base;
            v /= base;
            buf[i++] = (char)((d < 10) ? '0' + d : (upper ? 'A' : 'a') + (d - 10));
        }
    }
    int n = i;
    while (n < minw) {
        putc(pad);
        n++;
    }
    while (i)
        putc(buf[--i]);
}

int printf(const char *fmt, ...)
{
    va_list ap;
    va_start(ap, fmt);

    for (; *fmt; fmt++) {
        if (*fmt != '%') {
            putc(*fmt);
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
        if (*fmt == 'l') {
            is_long = 1;
            fmt++;
        }

        switch (*fmt) {
        case 'c':
            putc((char)va_arg(ap, int));
            break;
        case 's': {
            const char *s = va_arg(ap, const char *);
            if (!s)
                s = "(null)";
            while (*s)
                putc(*s++);
            break;
        }
        case 'd': {
            if (is_long) {
                long v = va_arg(ap, long);
                if (v < 0) { putc('-'); v = -v; }
                print_u64((u64)v, 10, 0, minw, pad);
            } else {
                int v = va_arg(ap, int);
                if (v < 0) { putc('-'); v = -v; }
                print_u64((u64)(unsigned)v, 10, 0, minw, pad);
            }
            break;
        }
        case 'u':
        case 'z': /* %zu = usize (u64 on rv64) */
            print_u64(is_long ? (u64)va_arg(ap, unsigned long)
                              : (u64)va_arg(ap, unsigned),
                      10, 0, minw, pad);
            break;
        case 'x':
            print_u64(is_long ? (u64)va_arg(ap, unsigned long)
                              : (u64)va_arg(ap, unsigned),
                      16, 0, minw, pad);
            break;
        case 'X':
            print_u64(is_long ? (u64)va_arg(ap, unsigned long)
                              : (u64)va_arg(ap, unsigned),
                      16, 1, minw, pad);
            break;
        case 'p':
            print_u64((u64)(uintptr_t)va_arg(ap, void *), 16, 0, 0, ' ');
            break;
        case 'b':
            print_u64(is_long ? (u64)va_arg(ap, unsigned long)
                              : (u64)va_arg(ap, unsigned),
                      2, 0, minw, pad);
            break;
        case '%':
            putc('%');
            break;
        default:
            putc('%');
            putc(*fmt);
            break;
        }
    }

    va_end(ap);
    return 0;
}
