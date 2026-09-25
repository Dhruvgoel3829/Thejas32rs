// src/main.rs
mod bus;
mod clint;
mod cpu;
mod dma;
mod gdb;
mod gpio;
mod i2c;
mod i2s;
mod plic;
mod spi;
mod timer;
mod uart;
mod wdt;
mod qspi;
mod ipc;

use bus::Bus;
use clap::Parser;
use cpu::Cpu;
use crossterm::terminal::{disable_raw_mode, enable_raw_mode};
use gdb::GdbStub;

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

    /// Enable GDB server on specified TCP port (e.g. 1234)
    #[arg(long)]
    gdb_port: Option<u16>,

    /// Enable IPC mode (Unix sockets for external simulator/sensor bridges)
    #[arg(long)]
    ipc: bool,
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

fn main() {
    let cli = Cli::parse();
    let mut bus = Bus::new(cli.ipc);

    eprintln!("[INFO] Loading ELF: {}", cli.elf);
    if cli.ipc {
        eprintln!("[INFO] IPC mode enabled (Unix sockets: /tmp/thejas_spi.sock, /tmp/thejas_gpio.sock)");
    }
    let entry_point = bus.load_elf(&cli.elf);
    eprintln!("[INFO] Entry point: 0x{:08x}", entry_point);

    let mut cpu = Cpu::new(bus, entry_point);
    cpu.write_reg(2, 0x8007_0000); // Set sp to top of SRAM

    let mut cycle_count: u64 = 0;

    if let Some(port) = cli.gdb_port {
        let mut gdb = GdbStub::wait_for_connection(port);

        // Halt on start and wait for GDB commands
        if !gdb.handle_client(&mut cpu) {
            return;
        }

        loop {
            cpu.step(cli.trace);

            if cpu.pc == 0 {
                gdb.notify_stop();
                break;
            }

            if gdb.is_at_breakpoint(cpu.pc) {
                gdb.notify_stop();
                if !gdb.handle_client(&mut cpu) {
                    break;
                }
            }
        }
    } else {
        // Standard non-GDB execution loop
        // UART uses Gazebo backend (no raw mode needed)

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
    }

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