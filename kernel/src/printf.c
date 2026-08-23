#include <stdarg.h>
#include <stdint.h>
#include <stddef.h>
#include "printf.h"
#include "uart.h"

static void putc(char c)
{
    uart_putc((uint8_t)c);
}

static void print_u64(uint64_t v, unsigned base, int upper, int minw, char pad)
{
    char buf[65];
    int i = 0, n;

    if (v == 0)
        buf[i++] = '0';
    while (v) {
        uint64_t d = v % base;
        v /= base;
        buf[i++] = (char)((d < 10) ? '0' + d : (upper ? 'A' : 'a') + (d - 10));
    }
    n = i;
    while (n < minw) {
        putc(pad);
        n++;
    }
    while (i)
        putc(buf[--i]);
}

static void print_int(long v, int minw, char pad)
{
    if (v < 0) {
        putc('-');
        v = -v;
    }
    print_u64((uint64_t)v, 10, 0, minw, pad);
}

static int vprintf_fmt(const char *fmt, va_list ap)
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
        int is_size_t = 0;
        if (*fmt == 'z') {
            is_size_t = 1;
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
            putc((char)va_arg(ap, int));
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
                print_u64((uint64_t)va_arg(ap, size_t), 10, 0, minw, pad);
            else if (is_long)
                print_u64((uint64_t)va_arg(ap, unsigned long), 10, 0, minw, pad);
            else
                print_u64((uint64_t)va_arg(ap, unsigned), 10, 0, minw, pad);
            count++;
            break;
        case 'x':
            if (is_long)
                print_u64((uint64_t)va_arg(ap, unsigned long), 16, 0, minw, pad);
            else
                print_u64((uint64_t)va_arg(ap, unsigned), 16, 0, minw, pad);
            count++;
            break;
        case 'X':
            if (is_long)
                print_u64((uint64_t)va_arg(ap, unsigned long), 16, 1, minw, pad);
            else
                print_u64((uint64_t)va_arg(ap, unsigned), 16, 1, minw, pad);
            count++;
            break;
        case 'p':
            print_u64((uint64_t)(uintptr_t)va_arg(ap, void *), 16, 0, 0, ' ');
            count++;
            break;
        case 'b':
            if (is_long)
                print_u64((uint64_t)va_arg(ap, unsigned long), 2, 0, minw, pad);
            else
                print_u64((uint64_t)va_arg(ap, unsigned), 2, 0, minw, pad);
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

int printf(const char *fmt, ...)
{
    va_list ap;
    int ret;

    va_start(ap, fmt);
    ret = vprintf_fmt(fmt, ap);
    va_end(ap);
    return ret;
}

void panic(const char *file, int line, const char *fmt, ...)
{
    va_list ap;

    uart_puts("PANIC [");
    uart_puts(file);
    uart_putc(':');
    print_int(line, 0, ' ');
    uart_puts("] ");

    va_start(ap, fmt);
    vprintf_fmt(fmt, ap);
    va_end(ap);

    uart_putc('\n');

    for (;;)
        asm volatile("wfi");
}
