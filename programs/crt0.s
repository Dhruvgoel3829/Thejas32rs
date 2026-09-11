.section .text.entry
.global _start
.type _start, @function

_start:
    # Set stack pointer to top of SRAM (512KB from 0x80000000)
    li sp, 0x80070000

    # Call C main function
    call main

    # Fallback infinite loop if main returns
1:  j 1b