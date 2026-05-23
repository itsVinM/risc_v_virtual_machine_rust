MEMORY {
  DRAM (rwx) : ORIGIN = 0x80000000, LENGTH = 128M
}

SECTIONS {
  .text : {
    *(.text.init)
    *(.text .text.*)
  } > DRAM
  
  .data : {
    *(.data .data.*)
  } > DRAM
  
  .bss : {
    *(.bss .bss.*)
  } > DRAM
}

ENTRY(_start)