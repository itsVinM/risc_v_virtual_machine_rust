.section .text.init
.global _start


_start:
    # set stack pointer to top of dram
    li sp, 0x87FFFFF0 

    #load trap handler address into t0 temp register
    la t0, _trap
    
    # write t0 into mtvec CSR (tells CPU where to jump on interrupt)
    csrw mtvec, t0

    # arm timer: read current timer
    li t1, 0x200BFF8    # MTIME address
    ld t2, 0(t1)        # current time - load from address in t1

    # set mtimecmp = mtime + 500 (fire in 500 cycles)
    li t3, 500
    add t2, t2, t3      # t2 = mtime + 500
    li t1, 0x2004000    # t1 MTIMECMP address
    sd t2, 0(t1)        # write tt2 to mtimecmp

    # enable timer interrupt in mie CSR
    li t0, 0x80         #bit 7 = machine timer interrupt
    csrs mie, t0        #set bit in mie

    #enable global interrupts in mstatus CSR
    li t0, 0x8          # bit 3 = machine interrupt enable
    csrs mstatus, t0    # set bit in mstatus

    # jump to c main function
    call main

    # if main returns - halt
    ebreak

#--------------------------------------
# Trap handler - ON when interrupts
#--------------------------------------
.global _trap

_trap:
    # save all caller-saved registers (call clobbers ra, t0-t6, a0-a7)
    addi sp, sp, -80
    sd ra,  0(sp)
    sd t0,  8(sp)
    sd t1, 16(sp)
    sd t2, 24(sp)
    sd t3, 32(sp)
    sd t4, 40(sp)
    sd t5, 48(sp)
    sd t6, 56(sp)
    sd a0, 64(sp)
    sd a1, 72(sp)

    # Re-arm timer for next interrupt
    li t0, 0x200BFF8    # MTIME address
    ld t1, 0(t0)        # read current time
    li t2, 500
    add t1, t1, t2      # t1 = mtime + 500
    li t0, 0x2004000    # MTIMECMP address
    sd t1, 0(t0)        # write new mtimecmp

    # call C handler function
    call handler_interrupt

    # restore all caller-saved registers
    ld ra,  0(sp)
    ld t0,  8(sp)
    ld t1, 16(sp)
    ld t2, 24(sp)
    ld t3, 32(sp)
    ld t4, 40(sp)
    ld t5, 48(sp)
    ld t6, 56(sp)
    ld a0, 64(sp)
    ld a1, 72(sp)
    addi sp, sp, 80

    # return from trap
    mret