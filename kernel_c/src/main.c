#include <stdint.h>

volatile uint64_t ticks = 0;

// call assembly trap handler
void handler_interrupt(void){
    ticks++; //asm rearm the timer
}

int main(void){
    while(ticks < 5){
        // spin waiting for interrupts 
    }

    // halt
    asm volatile("ebreak");

    return 0;
}