// src/main.rs
mod bus;
mod cpu;

use bus::Bus;
use cpu::Cpu;

fn main() {
    let mut bus = Bus::new();

    // addi x1, x0, 10 -> 0x00a00093
    // addi x2, x1, 5  -> 0x00508113
    // add  x3, x1, x2 -> 0x002081b3
    bus.write_u32(0x0, 0x00a00093);
    bus.write_u32(0x4, 0x00508113);
    bus.write_u32(0x8, 0x002081b3);

    let mut cpu = Cpu::new(bus, 0x0);

    cpu.step(); // runs addi x1, 10
    cpu.step(); // runs addi x2, 5
    cpu.step(); // runs add  x3, x1, x2

    println!("Register x1: {}", cpu.read_reg(1)); // 10
    println!("Register x2: {}", cpu.read_reg(2)); // 5
    println!("Register x3: {}", cpu.read_reg(3)); // 15
    println!("Final PC: {:#x}", cpu.pc);
}