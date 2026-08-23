#ifndef KERNEL_PRINTF_H
#define KERNEL_PRINTF_H

int  printf(const char *fmt, ...);
void panic(const char *file, int line, const char *fmt, ...);

#endif
