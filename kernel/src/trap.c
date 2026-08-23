#include <stdint.h>
#include "printf.h"
#include "uart.h"

#define MCAUSE_LEN 64

static const char *exception_name(uint64_t code)
{
    switch (code) {
    case 0:  return "inst addr misaligned";
    case 1:  return "inst access fault";
    case 2:  return "illegal instruction";
    case 3:  return "breakpoint";
    case 4:  return "load addr misaligned";
    case 5:  return "load access fault";
    case 6:  return "store addr misaligned";
    case 7:  return "store access fault";
    case 8:  return "ecall from U-mode";
    case 9:  return "ecall from S-mode";
    case 11: return "ecall from M-mode";
    case 12: return "inst page fault";
    case 13: return "load page fault";
    case 15: return "store page fault";
    default: return "unknown";
    }
}

__attribute__((interrupt))
void trap_handler(void)
{
    uint64_t cause, epc, tval;
    int is_interrupt;

    asm volatile("csrr %0, mcause" : "=r"(cause));
    asm volatile("csrr %0, mepc"  : "=r"(epc));
    asm volatile("csrr %0, mtval" : "=r"(tval));

    is_interrupt = (cause >> (MCAUSE_LEN - 1)) & 1;

    if (is_interrupt) {
        panic("trap", 0,
              "interrupt cause=%lu epc=%p tval=%p",
              cause & ~((uint64_t)1 << (MCAUSE_LEN - 1)),
              (void *)epc, (void *)tval);
    }

    panic("trap", 0,
          "%s cause=%lu epc=%p tval=%p",
          exception_name(cause), cause, (void *)epc, (void *)tval);
}
