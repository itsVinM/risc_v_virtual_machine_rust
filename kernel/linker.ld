OUTPUT_ARCH(riscv)
ENTRY(_start)

PHDRS {
    text PT_LOAD FLAGS(5); /* R E */
    data PT_LOAD FLAGS(6); /* R W */
}

SECTIONS {
    . = 0x80200000;

    _kernel_start = .;

    .text : {
        KEEP(*(.text.entry))
        *(.text .text.*)
    } :text

    .rodata : {
        *(.rodata .rodata.*)
        *(.srodata .srodata.*)
    } :text

    . = ALIGN(8);
    .data : {
        __global_pointer$ = . + 0x800;
        *(.sdata .sdata.*)
        *(.data .data.*)
    } :data

    _bss_start = .;
    .bss : {
        *(.sbss .sbss.*)
        *(.bss .bss.*)
        *(COMMON)
    } :data
    _bss_end = .;

    . = ALIGN(16);
    . += 0x10000;           /* 64 KiB kernel stack, grows down */
    _stack_top = .;
    . -= 8;
    _stack_canary = .;

    /* End of kernel image (incl. stack); frame arena starts here. */
    _kernel_end = .;
    _end = .;
}
