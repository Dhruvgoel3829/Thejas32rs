mod bus;
mod cpu;

use bus::Bus;
use cpu::Cpu;

fn main() {
    let mut bus = Bus::new();

    // Assembly equivalent:
    // 0x00: addi x1, x0, 5     (x1 = 5)
    // 0x04: addi x2, x0, 0     (x2 = sum accumulator = 0)
    // Loop:
    // 0x08: add  x2, x2, x1    (x2 += x1)
    // 0x0c: addi x1, x1, -1    (x1 -= 1)
    // 0x10: bne  x1, x0, -8    (branch to 0x08 if x1 != 0)

    bus.write_u32(0x00, 0x00500093); // addi x1, x0, 5
    bus.write_u32(0x04, 0x00000113); // addi x2, x0, 0
    bus.write_u32(0x08, 0x00110133); // add  x2, x2, x1
    bus.write_u32(0x0c, 0xfff08093); // addi x1, x1, -1
    bus.write_u32(0x10, 0xfe009ce3); // bne  x1, x0, -8

    let mut cpu = Cpu::new(bus, 0x0);

    // Run until the loop finishes (x1 == 0 and PC advances to 0x14)
    while cpu.pc < 0x14 {
        cpu.step();
    }

    println!("Loop completed.");
    println!("Register x1 (counter): {}", cpu.read_reg(1)); // Should be 0
    println!("Register x2 (sum 5+4+3+2+1): {}", cpu.read_reg(2)); // Should be 15
    println!("Final PC: {:#x}", cpu.pc);
}