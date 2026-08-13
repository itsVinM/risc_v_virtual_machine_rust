OUTPUT_ARCH(riscv)
ENTRY(_start)

PHDRS {
    text PT_LOAD FLAGS(5); /* R E */
    data PT_LOAD FLAGS(6); /* R W */
}

SECTIONS {
    . = 0x80200000;

    .text : {
        *(.text.entry)
        *(.text .text.*)
    } :text

    .rodata : {
        *(.rodata .rodata.*)
    } :text

    .data : {
        *(.data .data.*)
    } :data

    .bss : {
        *(.bss .bss.*)
        *(COMMON)
    } :data

    . = ALIGN(16);
    . += 0x10000;           /* 64 KiB kernel stack, grows down */
    _stack_top = .;
}
