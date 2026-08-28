.section .text.entry
.global _start
.global trap_entry
.global start_first_task
.type _start, @function
.type trap_entry, @function
.type start_first_task, @function

_start:
    # 1. Disable all interrupts during boot
    csrw    mstatus, zero
    csrw    mie, zero

    # 2. Set Direct-Mode trap vector to trap_entry
    la      t0, trap_entry
    csrw    mtvec, t0

    # 3. Set temporary boot stack and call kernel init
    li      sp, 0x80070000
    call    scheduler_init

    # 4. Jump into the first scheduled task
    call    start_first_task

    # Loop forever if task ever returns
1:  j       1b


.align 4
trap_entry:
    # Allocate frame on current task's stack
    addi    sp, sp, -128

    # Save caller & callee saved registers
    sw      x1,   0(sp)
    sw      x3,   8(sp)
    sw      x4,  12(sp)
    sw      x5,  16(sp)
    sw      x6,  20(sp)
    sw      x7,  24(sp)
    sw      x8,  28(sp)
    sw      x9,  32(sp)
    sw      x10, 36(sp)
    sw      x11, 40(sp)
    sw      x12, 44(sp)
    sw      x13, 48(sp)
    sw      x14, 52(sp)
    sw      x15, 56(sp)
    sw      x16, 60(sp)
    sw      x17, 64(sp)
    sw      x18, 68(sp)
    sw      x19, 72(sp)
    sw      x20, 76(sp)
    sw      x21, 80(sp)
    sw      x22, 84(sp)
    sw      x23, 88(sp)
    sw      x24, 92(sp)
    sw      x25, 96(sp)
    sw      x26, 100(sp)
    sw      x27, 104(sp)
    sw      x28, 108(sp)
    sw      x29, 112(sp)
    sw      x30, 116(sp)
    sw      x31, 120(sp)

    # Save mepc (interrupted PC)
    csrr    t0, mepc
    sw      t0, 124(sp)

    # Pass current SP to C scheduler via a0, returns next task SP in a0
    mv      a0, sp
    call    scheduler_switch
    mv      sp, a0

    # Restore mepc
    lw      t0, 124(sp)
    csrw    mepc, t0

    # Restore registers
    lw      x1,   0(sp)
    lw      x3,   8(sp)
    lw      x4,  12(sp)
    lw      x5,  16(sp)
    lw      x6,  20(sp)
    lw      x7,  24(sp)
    lw      x8,  28(sp)
    lw      x9,  32(sp)
    lw      x10, 36(sp)
    lw      x11, 40(sp)
    lw      x12, 44(sp)
    lw      x13, 48(sp)
    lw      x14, 52(sp)
    lw      x15, 56(sp)
    lw      x16, 60(sp)
    lw      x17, 64(sp)
    lw      x18, 68(sp)
    lw      x19, 72(sp)
    lw      x20, 76(sp)
    lw      x21, 80(sp)
    lw      x22, 84(sp)
    lw      x23, 88(sp)
    lw      x24, 92(sp)
    lw      x25, 96(sp)
    lw      x26, 100(sp)
    lw      x27, 104(sp)
    lw      x28, 108(sp)
    lw      x29, 112(sp)
    lw      x30, 116(sp)
    lw      x31, 120(sp)

    # Pop stack frame
    addi    sp, sp, 128

    # Return to next task context
    mret


start_first_task:
    # Load first task SP from current_task_sp
    la      t0, current_sp
    lw      sp, 0(t0)

    # Restore mepc for Task 1
    lw      t0, 124(sp)
    csrw    mepc, t0

    # Set mstatus.MPIE (bit 7) and mstatus.MPP (bits 12:11 = Machine mode)
    li      t0, (1 << 7) | (3 << 11)
    csrs    mstatus, t0

    # Pop initial frame
    addi    sp, sp, 128

    # Start Task 1 with interrupts enabled on entry
    mret
