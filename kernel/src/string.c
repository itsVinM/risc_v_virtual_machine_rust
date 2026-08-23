#include "string.h"
#include <stdint.h>

void *memset(void *s, int c, size_t n)
{
    uint8_t *p = s;
    uint64_t word;
    int i;

    /* align to word boundary */
    while (n && ((uintptr_t)p & 7)) {
        *p++ = (uint8_t)c;
        n--;
    }

    /* build a word filled with the target byte */
    word = 0;
    for (i = 0; i < 8; i++)
        word = (word << 8) | (uint8_t)c;

    /* store whole words */
    {
        uint64_t *wp = (uint64_t *)p;
        size_t words = n / 8;
        while (words--)
            *wp++ = word;
        p = (uint8_t *)wp;
    }

    /* store trailing bytes */
    n %= 8;
    while (n--)
        *p++ = (uint8_t)c;

    return s;
}

void *memcpy(void *dst, const void *src, size_t n)
{
    uint8_t *d = dst;
    const uint8_t *s = src;

    /* align destination to word boundary */
    while (n && ((uintptr_t)d & 7)) {
        *d++ = *s++;
        n--;
    }

    /* copy whole words */
    {
        uint64_t *dw = (uint64_t *)d;
        const uint64_t *sw = (const uint64_t *)s;
        size_t words = n / 8;
        while (words--)
            *dw++ = *sw++;
        d = (uint8_t *)dw;
        s = (const uint8_t *)sw;
    }

    /* copy trailing bytes */
    n %= 8;
    while (n--)
        *d++ = *s++;

    return dst;
}
