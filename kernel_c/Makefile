CC      = riscv64-unknown-elf-gcc
OBJCOPY = riscv64-unknown-elf-objcopy
CFLAGS  = -march=rv64im -mabi=lp64 -nostdlib -nostartfiles -O0

TARGET  = kernel

all: $(TARGET).elf

$(TARGET).elf: src/entry.S src/main.c
	$(CC) $(CFLAGS) -T linker.ld -o $@ $^

clean:
	rm -f $(TARGET).elf
