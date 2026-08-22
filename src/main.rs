mod bus;
mod clint;
mod cpu;
mod uart;

use bus::Bus;
use cpu::Cpu;
use std::env;

fn main() {
    let args: Vec<String> = env::args().collect();
    if args.len() < 2 {
        eprintln!("Usage: cargo run -- <path-to-elf-file>");
        return;
    }

    let elf_path = &args[1];
    let mut bus = Bus::new();

    println!("[INFO] Loading ELF: {}", elf_path);
    let entry_point = bus.load_elf(elf_path);
    println!("[INFO] Entry point: {:#x}", entry_point);

    let mut cpu = Cpu::new(bus, entry_point);

    // Initialize Stack Pointer (sp = x2) near the top of SRAM (0x8007_0000)
    cpu.write_reg(2, 0x8007_0000);

    // Run execution loop
    loop {
        cpu.step();

        // Break on unaligned PC or zero instruction as safety guard
        if cpu.pc == 0 {
            println!("\n[INFO] Target halted (PC = 0).");
            break;
        }
    }
}