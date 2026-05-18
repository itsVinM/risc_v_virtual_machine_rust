OUTPUT_ARCH(riscv)
ENTRY(_start)

MEMORY {
    dram : ORIGIN = 0x80000000, LENGTH = 128M
}

SECTIONS {
    . = 0x80000000;

    .text.init : { *(.text.init) } > dram
    .text      : { *(.text .text.*) } > dram
    .rodata    : { *(.rodata .rodata.*) } > dram
    .data      : { *(.data .data.*) } > dram
    .bss       : {
        _bss_start = .;
        *(.bss .bss.*)
        _bss_end = .;
    } > dram
}
