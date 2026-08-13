# kernel/ — freestanding RISC-V kernel, pure C (no assembly). Booted by the
# rv64vm VM:  make vm ARGS="--sbi kernel/kernel"
#
#   make            build kernel/kernel (ELF, linked at 0x80200000)
#   make clean

CC := riscv64-linux-gnu-gcc
CFLAGS := -march=rv64ima -mabi=lp64 -mcmodel=medany \
	-O2 -Wall -Wextra -Werror -ffreestanding -fno-builtin -fno-stack-protector \
	-fno-pic -fno-pie -nostdlib -static
LDFLAGS := -nostdlib -static -T linker.ld -Wl,--build-id=none

SRCS := kernel.c uart.c printf.c mem.c

.PHONY: all clean

all: kernel

kernel: $(SRCS) linker.ld
	$(CC) $(CFLAGS) $(LDFLAGS) -o $@ $(SRCS)

clean:
	rm -f kernel
