.section .text.entry
.global _start
.type _start, @function

_start:
    # Set stack pointer from linker symbol (top of SRAM)
    la sp, __stack_top

    # Zero BSS (bounded >= comparison: safe even if bounds were unaligned)
    la a0, __bss_start
    la a1, __bss_end
    bgeu a0, a1, 1f
    li a2, 0
0:
    sw a2, 0(a0)
    addi a0, a0, 4
    bltu a0, a1, 0b

1:
    # Call C main function
    call main

    # Fallback infinite loop if main returns
2:  j 2b
