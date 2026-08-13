#ifndef KERNEL_UART_H
#define KERNEL_UART_H

#include <stdint.h>

void uart_init(void);
void uart_putc(uint8_t c);

#endif
