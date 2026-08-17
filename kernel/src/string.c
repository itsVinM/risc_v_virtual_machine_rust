#include "string.h"

void *memset(void *s, int c, usize n)
{
    u8 *p = s;
    while (n && ((uintptr_t)p & 7)) {
        *p++ = (u8)c;
        n--;
    }
    u64 word = 0x0101010101010101ULL * (u8)c;
    u64 *wp = (u64 *)p;
    while (n >= 8) {
        *wp++ = word;
        n -= 8;
    }
    p = (u8 *)wp;
    while (n--)
        *p++ = (u8)c;
    return s;
}

void *memcpy(void *dst, const void *src, usize n)
{
    u8 *d = dst;
    const u8 *s = src;
    while (n--)
        *d++ = *s++;
    return dst;
}

void *memmove(void *dst, const void *src, usize n)
{
    u8 *d = dst;
    const u8 *s = src;
    if (d < s) {
        while (n--)
            *d++ = *s++;
    } else {
        d += n;
        s += n;
        while (n--)
            *--d = *--s;
    }
    return dst;
}
