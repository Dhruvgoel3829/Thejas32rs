OUTPUT_ARCH("riscv")
ENTRY(_start)

MEMORY
{
    RAM (rwx) : ORIGIN = 0x80000000, LENGTH = 256K
}

SECTIONS
{
    .text : {
        __text_start = .;
        KEEP(*(.text.entry))
        *(.text .text.*)
        __text_end = .;
    } > RAM

    .rodata : {
        . = ALIGN(4);
        *(.rodata .rodata.*)
        *(.srodata .srodata.*)
    } > RAM

    .data : {
        __data_start = .;
        . = ALIGN(4);
        *(.data .data.*)
        *(.sdata .sdata.*)
        __data_end = .;
    } > RAM

    .bss : {
        . = ALIGN(4);
        __bss_start = .;
        *(.bss .bss.*)
        *(.sbss .sbss.*)
        *(COMMON)
        . = ALIGN(4);
        __bss_end = .;
    } > RAM

    . = ALIGN(16);
    _end = .;
    __stack_top = ORIGIN(RAM) + LENGTH(RAM);
}