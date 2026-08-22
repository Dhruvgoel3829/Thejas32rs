// src/main.rs
mod bus;
mod cpu;
mod uart;

use bus::Bus;
use cpu::Cpu;

fn main() {
    let mut bus = Bus::new();

    // 1. Put the string "Hello, THEJAS32!\n\0" at address 0x200
    let message = b"Hello, THEJAS32!\n\0";
    for (i, &byte) in message.iter().enumerate() {
        bus.write_u8(0x200 + i as u32, byte);
    }

    // 2. Machine code loop to print string to UART (0x1000_0000):
    // 0x00: lui  x1, 0x10000      (x1 = UART_BASE = 0x10000000)
    // 0x04: addi x2, x0, 0x200    (x2 = pointer to string = 0x200)
    // Loop:
    // 0x08: lb   x3, 0(x2)        (x3 = *x2)
    // 0x0c: beq  x3, x0, 16       (if x3 == 0 goto Exit at 0x1c)
    // 0x10: sb   x3, 0(x1)        (UART_THR = x3 -> writes char to stdout)
    // 0x14: addi x2, x2, 1        (x2++)
    // 0x18: jal  x0, -16          (jump back to 0x08)
    // Exit:
    // 0x1c: ecall

    bus.write_u32(0x00, 0x100000b7); // lui  x1, 0x10000
    bus.write_u32(0x04, 0x20000113); // addi x2, x0, 0x200
    bus.write_u32(0x08, 0x00010183); // lb   x3, 0(x2)
    bus.write_u32(0x0c, 0x00018863); // beq  x3, x0, 16 (offset +16 -> 0x1c)
    bus.write_u32(0x10, 0x00308023); // sb   x3, 0(x1)
    bus.write_u32(0x14, 0x00110113); // addi x2, x2, 1
    bus.write_u32(0x18, 0xff1ff06f); // jal  x0, -16 (offset -16 -> 0x08)

    let mut cpu = Cpu::new(bus, 0x0);

    println!("--- Emulating THEJAS32 UART Output ---");
    while cpu.pc < 0x1c {
        cpu.step();
    }
    println!("--- Execution Finished ---");
}