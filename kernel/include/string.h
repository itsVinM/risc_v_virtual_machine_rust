#ifndef STRING_H
#define STRING_H

#include "types.h"

void *memset(void *s, int c, usize n);
void *memcpy(void *dst, const void *src, usize n);
void *memmove(void *dst, const void *src, usize n);

#endif
