# kernel/ — freestanding RISC-V kernel, C++20 (no assembly). Booted by the
# rv64vm VM:  make vm ARGS="--sbi kernel/build/kernel.elf"
#
#   make            build kernel/build/kernel.elf (ELF, linked at 0x80200000)
#   make clean

TOOLCHAINS := riscv64-unknown-elf riscv64-elf riscv64-linux-gnu riscv64-none-elf
TOOLCHAIN := $(firstword $(foreach tc,$(TOOLCHAINS),\
	$(if $(shell command -v $(tc)-g++ 2>/dev/null),$(tc),)))

ifeq ($(TOOLCHAIN),)
$(error no RISC-V g++ cross-toolchain found in PATH)
endif

CXX := $(TOOLCHAIN)-g++
OBJDIR := build

CXXFLAGS := -std=c++20 \
	-march=rv64ima_zicsr_zifencei -mabi=lp64 -mcmodel=medany -msmall-data-limit=0 \
	-O2 -g -Wall -Wextra -Werror \
	-ffreestanding -fno-builtin -fno-stack-protector -fno-pic -fno-pie -nostdlib -static \
	-fno-exceptions -fno-rtti -fno-unwind-tables -fno-asynchronous-unwind-tables \
	-Iinclude
LDFLAGS := -nostdlib -static -T linker.ld -Wl,--build-id=none

SRCS := kernel.cpp uart.cpp printf.cpp mem.cpp string.cpp sbi.cpp trap.cpp dtb.cpp panic.cpp mmu.cpp
OBJS := $(addprefix $(OBJDIR)/,$(SRCS:.cpp=.o))

.PHONY: all clean

all: $(OBJDIR)/kernel.elf

$(OBJDIR)/kernel.elf: $(OBJS) linker.ld | $(OBJDIR)
	$(CXX) $(LDFLAGS) -o $@ $(OBJS)

$(OBJDIR)/%.o: src/%.cpp Makefile | $(OBJDIR)
	$(CXX) $(CXXFLAGS) -c $< -o $@

$(OBJDIR):
	mkdir -p $(OBJDIR)

clean:
	rm -rf $(OBJDIR) kernel.elf kernel
