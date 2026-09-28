.section .text.entry
.global _start
.type _start, @function

_start:
    # Set stack pointer to top of SRAM (256KB from 0x80000000)
    li sp, 0x80040000

    # Zero BSS
    la a0, __bss_start
    la a1, __bss_end
    beq a0, a1, 1f
    li a2, 0
0:
    sw a2, 0(a0)
    addi a0, a0, 4
    bne a0, a1, 0b

1:
    # Call C main function
    call main

    # Fallback infinite loop if main returns
2:  j 2b