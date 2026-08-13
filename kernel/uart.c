#include "uart.h"

/* 8250 UART on the virt platform, MMIO only. */
#define UART_BASE 0x10000000UL
#define UART_THR 0
#define UART_LSR 5
#define UART_LSR_TX_EMPTY (1u << 5)

static volatile uint8_t *const uart = (volatile uint8_t *)UART_BASE;

void uart_init(void)
{
}

void uart_putc(uint8_t c)
{
    while (!(uart[UART_LSR] & UART_LSR_TX_EMPTY))
        ;
    uart[UART_THR] = c;
}
