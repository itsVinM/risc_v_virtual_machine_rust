#include <stdint.h>

#define MTIME_ADDR      0x200BFF8
#define MTIMECMP_ADDR   0x2004000
#define TIMER_INTERVAL  500

volatile uint64_t ticks = 0;

static inline uint64_t read_u64(uint64_t addr) {
    uint64_t val;
    asm volatile("ld %0, 0(%1)" : "=r"(val) : "r"(addr));
    return val;
}

static inline void write_u64(uint64_t addr, uint64_t val) {
    asm volatile("sd %0, 0(%1)" : : "r"(val), "r"(addr));
}

static inline void csrs(uint64_t csr_num, uint64_t val) {
    asm volatile("csrs %0, %1" : : "i"(csr_num), "r"(val));
}

void handler_interrupt(void) {
    ticks++;
}

int main(void) {
    uint64_t trap_addr;
    uint64_t current_time;
    uint64_t next_interrupt;

    asm volatile("la %0, _trap" : "=r"(trap_addr));
    asm volatile("csrw mtvec, %0" : : "r"(trap_addr));

    current_time = read_u64(MTIME_ADDR);
    next_interrupt = current_time + TIMER_INTERVAL;
    write_u64(MTIMECMP_ADDR, next_interrupt);

    csrs(0x304, (1 << 7));   // mie.MTIE
    csrs(0x300, (1 << 3));   // mstatus.MIE

    while (ticks < 5) { }

    asm volatile("ebreak");
    return 0;
}
