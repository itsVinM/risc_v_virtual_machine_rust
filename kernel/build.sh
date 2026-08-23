#!/bin/sh
set -e

CC=riscv64-elf-gcc
ARCH="-march=rv64ima_zicsr_zifencei -mabi=lp64 -mcmodel=medany"
WARN="-Wall -Wextra -Werror"
OPT="-O2 -g"
FR="-ffreestanding -fno-builtin -fno-stack-protector -fno-pic -fno-pie -nostdlib -static"
INC="-Iinclude"

rm -f kernel.o uart.o printf.o mem.o string.o sbi.o trap.o kernel.elf

$CC $ARCH $WARN $OPT $FR $INC -c src/kernel.c -o kernel.o
$CC $ARCH $WARN $OPT $FR $INC -c src/uart.c -o uart.o
$CC $ARCH $WARN $OPT $FR $INC -c src/printf.c -o printf.o
$CC $ARCH $WARN $OPT $FR $INC -c src/mem.c -o mem.o
$CC $ARCH $WARN $OPT $FR $INC -c src/string.c -o string.o
$CC $ARCH $WARN $OPT $FR $INC -c src/sbi.c -o sbi.o
$CC $ARCH $WARN $OPT $FR $INC -c src/trap.c -o trap.o

echo "=== All .o files compiled ==="

$CC -nostdlib -static -T linker.ld -Wl,--build-id=none -o kernel.elf kernel.o uart.o printf.o mem.o string.o sbi.o trap.o

echo "=== kernel.elf linked ==="
riscv64-elf-size kernel.elf
riscv64-elf-objdump -d kernel.elf | head -80
