#ifndef KERNEL_UART_H
#define KERNEL_UART_H

#include <stdint.h>

void uart_init(void);
void uart_init_from_dtb(const void *dtb);
void uart_putc(uint8_t c);
void uart_puts(const char *s);

#endif
