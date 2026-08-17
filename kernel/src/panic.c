#include "panic.h"
#include "printf.h"

void panic(const char *file, int line, const char *msg)
{
    printf("PANIC %s:%d: %s\n", file, line, msg);
    for (;;)
        asm volatile("wfi");
}
