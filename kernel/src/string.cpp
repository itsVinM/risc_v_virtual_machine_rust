#include "string.hpp"

#include <stdint.h>

// Kept at global scope with C linkage: GCC is allowed to emit implicit
// memset/memcpy calls even under -ffreestanding (block moves, zero-init).

extern "C" void *memset(void *s, int c, size_t n)
{
    auto *p = static_cast<unsigned char *>(s);

    /* align to word boundary */
    while (n && (reinterpret_cast<uintptr_t>(p) & 7)) {
        *p++ = static_cast<unsigned char>(c);
        n--;
    }

    /* build a word filled with the target byte */
    uint64_t word = 0;
    for (int i = 0; i < 8; i++)
        word = (word << 8) | static_cast<uint8_t>(c);

    /* store whole words */
    {
        auto *wp = reinterpret_cast<uint64_t *>(p);
        size_t words = n / 8;
        while (words--)
            *wp++ = word;
        p = reinterpret_cast<unsigned char *>(wp);
    }

    /* store trailing bytes */
    n %= 8;
    while (n--)
        *p++ = static_cast<unsigned char>(c);

    return s;
}

extern "C" void *memcpy(void *dst, const void *src, size_t n)
{
    auto *d = static_cast<unsigned char *>(dst);
    const auto *s = static_cast<const unsigned char *>(src);

    /* align destination to word boundary */
    while (n && (reinterpret_cast<uintptr_t>(d) & 7)) {
        *d++ = *s++;
        n--;
    }

    /* copy whole words */
    {
        auto *dw = reinterpret_cast<uint64_t *>(d);
        const auto *sw = reinterpret_cast<const uint64_t *>(s);
        size_t words = n / 8;
        while (words--)
            *dw++ = *sw++;
        d = reinterpret_cast<unsigned char *>(dw);
        s = reinterpret_cast<const unsigned char *>(sw);
    }

    /* copy trailing bytes */
    n %= 8;
    while (n--)
        *d++ = *s++;

    return dst;
}
