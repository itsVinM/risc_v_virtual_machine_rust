# ASM instruction set

li reg, value            - load immediate , put a number into a register
la reg, label            - load address, put memory address of a label into register
ld reg, offset(addr_reg) - load 64-bit value from memoru at address

sd reg, offset(addr_reg) - store 64-bit value to memory
add dst, src1, src2      - arithmetic: dst = src1 + src2

csrw csr, reg            - write CSR (control register)
csrs csr, reg            - set bits in CSR

call label               - call function (jump + save return address)
mret                     - return from trap (jump back to where interrupt happened)
