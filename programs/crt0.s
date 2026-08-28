.section .text.entry
.global _start
.type _start, @function

_start:
    # 1. Disable all interrupts during early boot
    csrw    mstatus, zero
    csrw    mie, zero

    # 2. Set Stack Pointer to top of SRAM (512 KB offset)
    li      sp, 0x80070000

    # 3. Call C main()
    call    main

    # 4. If main returns, jump to 0 to halt emulator
    jr      zero

.size _start, . - _start
