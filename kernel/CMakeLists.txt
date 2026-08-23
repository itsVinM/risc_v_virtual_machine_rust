cmake_minimum_required(VERSION 3.16)

# ── Bare-metal RISC-V cross setup (must precede project()) ───────────────
set(CMAKE_SYSTEM_NAME Generic)      # no OS: disables host (Apple) link flags
set(CMAKE_SYSTEM_PROCESSOR riscv64)

find_program(RISCV_GXX
    NAMES riscv64-elf-g++ riscv64-unknown-elf-g++ riscv64-none-elf-g++ riscv64-linux-gnu-g++)
if(NOT RISCV_GXX)
    message(FATAL_ERROR "no RISC-V g++ cross-toolchain found in PATH")
endif()
set(CMAKE_CXX_COMPILER ${RISCV_GXX})

# No libc/libstdc++ here: keep the compiler sanity check compile-only.
set(CMAKE_TRY_COMPILE_TARGET_TYPE STATIC_LIBRARY)

project(rv64vm-kernel LANGUAGES CXX)

set(CMAKE_CXX_STANDARD 20)
set(CMAKE_CXX_STANDARD_REQUIRED ON)
set(CMAKE_CXX_EXTENSIONS OFF)

if(NOT CMAKE_BUILD_TYPE)
    set(CMAKE_BUILD_TYPE Release)
endif()
set(CMAKE_CXX_FLAGS_RELEASE "-O2 -g")

add_executable(kernel.elf
    src/kernel.cpp
    src/uart.cpp
    src/printf.cpp
    src/mem.cpp
    src/string.cpp
    src/sbi.cpp
    src/trap.cpp
    src/dtb.cpp
    src/panic.cpp
    src/mmu.cpp)

target_include_directories(kernel.elf PRIVATE include)

target_compile_options(kernel.elf PRIVATE
    -march=rv64ima_zicsr_zifencei
    -mabi=lp64
    -mcmodel=medany
    -msmall-data-limit=0
    -ffreestanding
    -fno-builtin
    -fno-stack-protector
    -fno-pic
    -fno-pie
    -fno-exceptions
    -fno-rtti
    -fno-unwind-tables
    -fno-asynchronous-unwind-tables
    -Wall
    -Wextra
    -Werror)

target_link_options(kernel.elf PRIVATE
    -nostdlib
    -static
    -T ${CMAKE_CURRENT_SOURCE_DIR}/linker.ld
    -Wl,--build-id=none)
