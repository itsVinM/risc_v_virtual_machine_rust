OUTPUT_ARCH(riscv)
ENTRY(_entry)

MEMORY
{
    DRAM (rwx) : ORIGIN = 0x80000000, LENGTH = 128M
}

BASE_ADDRESS = 0x80200000;

SECTIONS
{
    . = BASE_ADDRESS;

    _kernel_start = .;

    .text : AT(ADDR(.text)) {
        _text_start = .;
        *(.text.entry)
        *(.text .text.*)
        . = ALIGN(4K);
        _text_end = .;
    }

    .rodata : AT(ADDR(.rodata)) {
        _rodata_start = .;
        *(.rodata .rodata.*)
        . = ALIGN(4K);
        _rodata_end = .;
    }

    .data : AT(ADDR(.data)) {
        _data_start = .;
        *(.data .data.*)
        . = ALIGN(4K);
        _data_end = .;
    }

    .bss : AT(ADDR(.bss)) {
        _bss_start = .;
        *(.bss .bss.*)
        *(.sbss .sbss.*)
        *(COMMON)
        . = ALIGN(4K);
        _bss_end = .;
    }

    _kernel_end = .;

    . = ALIGN(4K);
    _heap_start = .;
    _heap_end = ORIGIN(DRAM) + LENGTH(DRAM) - 16K;
    _stack_top = ORIGIN(DRAM) + LENGTH(DRAM) - 8K;
    _stack_bottom = ORIGIN(DRAM) + LENGTH(DRAM) - 16K;
}

PROVIDE(_sdata = _data_start);
PROVIDE(_edata = _data_end);
PROVIDE(_sbss = _bss_start);
PROVIDE(_ebss = _bss_end);
PROVIDE(_sidata = _rodata_end);
