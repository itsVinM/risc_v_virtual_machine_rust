#include "uart.h"

#define UART_BASE 0x10000000UL
#define UART_THR  0
#define UART_RBR  0
#define UART_LSR  5
#define UART_LSR_TX_EMPTY (1u << 5)
#define UART_LSR_RX_READY (1u << 0)

static volatile u8 *const uart = (volatile u8 *)UART_BASE;

void uart_init(void)
{
}

void uart_putc(u8 c)
{
    while (!(uart[UART_LSR] & UART_LSR_TX_EMPTY))
        ;
    uart[UART_THR] = c;
}

int uartgetc(void)
{
    if (!(uart[UART_LSR] & UART_LSR_RX_READY))
        return -1;
    return (int)uart[UART_RBR];
}

u8 uart_getc(void)
{
    int c;
    while ((c = uartgetc()) == -1)
        ;
    return (u8)c;
}
