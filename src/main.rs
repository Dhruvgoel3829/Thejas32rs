// src/main.rs
mod bus;
mod cpu;

use bus::Bus;
use cpu::Cpu;

fn main() {
    let mut bus = Bus::new();

    // 0x00: csrw mtvec, x1 (set trap vector to 0x100) -> csrrw x0, mtvec, x1
    // 0x04: ecall          (triggers trap to mtvec=0x100)
    // 0x08: addi x2, x0, 77 (runs after mret returns)
    //
    // --- Trap Handler at 0x100 ---
    // 0x100: addi mepc, mepc, 4 (advance mepc past ecall instruction)
    // 0x104: mret               (return to mepc)

    // Setup x1 = 0x100 (trap handler address)
    bus.write_u32(0x00, 0x10000093); // addi x1, x0, 0x100
    bus.write_u32(0x04, 0x30509073); // csrrw x0, mtvec(0x305), x1
    bus.write_u32(0x08, 0x00000073); // ecall
    bus.write_u32(0x0c, 0x04d00113); // addi x2, x0, 77

    // Trap handler at 0x100:
    // Read mepc, add 4, write back, then mret
    bus.write_u32(0x100, 0x34102173); // csrr x2, mepc (csrs x2, mepc, x0)
    bus.write_u32(0x104, 0x00410113); // addi x2, x2, 4
    bus.write_u32(0x108, 0x34111073); // csrw mepc, x2
    bus.write_u32(0x10c, 0x30200073); // mret

    let mut cpu = Cpu::new(bus, 0x0);

    for _ in 0..8 {
        cpu.step();
    }

    println!("Register x2 (after trap return): {}", cpu.read_reg(2)); // 77
    println!("Final PC: {:#x}", cpu.pc);
}