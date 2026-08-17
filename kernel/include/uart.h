#ifndef UART_H
#define UART_H

#include "types.h"

void    uart_init(void);
void    uart_putc(u8 c);
u8      uart_getc(void);
int     uartgetc(void); /* -1 if nothing */

#endif
