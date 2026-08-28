mod bus;
mod clint;
mod cpu;
mod gpio;
mod i2c;
mod i2s;
mod plic;
mod spi;
mod timer;
mod uart;

use bus::Bus;
use clap::Parser;
use cpu::Cpu;
use crossterm::terminal::{disable_raw_mode, enable_raw_mode};

#[derive(Parser, Debug)]
#[command(name = "Thejas32rs")]
#[command(author = "Dhruv Goel")]
#[command(version = "0.1.0")]
#[command(about = "Cycle-accurate RV32IM THEJAS32 SoC Emulator in Rust", long_about = None)]
struct Cli {
    /// Path to the RISC-V ELF binary
    #[arg(value_name = "FILE")]
    elf: String,

    /// Print instruction trace disassembly and register state during execution
    #[arg(short, long)]
    trace: bool,

    /// Dump registers on exit / crash
    #[arg(short, long)]
    dump_regs: bool,

    /// Limit maximum execution cycles (0 for unlimited)
    #[arg(short, long, default_value_t = 0)]
    max_cycles: u64,
}

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

// src/main.rs
fn main() {
    let cli = Cli::parse();
    let mut bus = Bus::new();

    eprintln!("[INFO] Loading ELF: {}", cli.elf);
    let entry_point = bus.load_elf(&cli.elf);
    eprintln!("[INFO] Entry point: 0x{:08x}", entry_point);

    let mut cpu = Cpu::new(bus, entry_point);
    cpu.write_reg(2, 0x8007_0000); // sp

    let mut cycle_count: u64 = 0;

    // Run execution inside a scoped block so raw mode drops before final logs
    {
        let _raw_guard = RawModeGuard::enter();

        loop {
            cpu.step(cli.trace);
            cycle_count += 1;

            if cpu.pc == 0 {
                break; // Clean shutdown when firmware jumps to 0
            }

            if cli.max_cycles > 0 && cycle_count >= cli.max_cycles {
                break;
            }
        }
    } // <-- Raw mode is disabled here automatically!

    if cli.max_cycles > 0 && cycle_count >= cli.max_cycles {
        println!(
            "\n[INFO] Reached max cycle limit ({} cycles)",
            cli.max_cycles
        );
    }

    if cli.dump_regs {
        cpu.dump_registers();
    }
}
