mod bus;
mod clint;
mod cpu;
mod uart;

use bus::Bus;
use cpu::Cpu;
use std::env;
use crossterm::terminal::{enable_raw_mode, disable_raw_mode};

struct RawModeGuard;

impl RawModeGuard {
    pub fn enter() -> Self {
        enable_raw_mode().expect("Failed to enable raw mode");
        Self
    }
}

impl Drop for RawModeGuard {
    fn drop(&mut self) {
        let _ = disable_raw_mode();
    }
}

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
    cpu.write_reg(2, 0x8007_0000); // Initialize Stack Pointer (sp)

    // Activate raw terminal mode
    let _raw_guard = RawModeGuard::enter();

    loop {
        cpu.step();

        if cpu.pc == 0 {
            break;
        }
    }
}