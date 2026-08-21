pub mod decode;

use crate::bus::Bus;
use decode::Instruction;

pub struct Cpu {
    pub regs: [u32; 32],
    pub pc: u32,
    pub bus: Bus,
}

impl Cpu {
    pub fn new(bus: Bus, entry_point: u32) -> Self {
        Self {
            regs: [0; 32],
            pc: entry_point,
            bus,
        }
    }

    #[inline(always)]
    pub fn read_reg(&self, reg: usize) -> u32 {
        if reg == 0 { 0 } else { self.regs[reg] }
    }

    #[inline(always)]
    pub fn write_reg(&mut self, reg: usize, val: u32) {
        if reg != 0 {
            self.regs[reg] = val;
        }
    }

    pub fn step(&mut self) {
        let raw = self.bus.read_u32(self.pc);
        let inst = Instruction(raw);
        let mut next_pc = self.pc.wrapping_add(4);

        match inst.opcode() {
            // LUI
            0x37 => {
                self.write_reg(inst.rd(), inst.imm_u());
            }

            // AUIPC
            0x17 => {
                let val = self.pc.wrapping_add(inst.imm_u());
                self.write_reg(inst.rd(), val);
            }

            // OP-IMM (ADDI, SLTI, ANDI, ORI, XORI, SLLI, SRLI, SRAI)
            0x13 => {
                let src = self.read_reg(inst.rs1());
                let imm = inst.imm_i();
                let res = match inst.funct3() {
                    0x0 => src.wrapping_add(imm), // ADDI
                    0x2 => {
                        if (src as i32) < (imm as i32) {
                            1
                        } else {
                            0
                        }
                    } // SLTI
                    0x3 => {
                        if src < imm {
                            1
                        } else {
                            0
                        }
                    } // SLTIU
                    0x4 => src ^ imm,             // XORI
                    0x6 => src | imm,             // ORI
                    0x7 => src & imm,             // ANDI
                    0x1 => src << (imm & 0x1F),   // SLLI
                    0x5 => {
                        if (inst.funct7() & 0x20) != 0 {
                            ((src as i32) >> (imm & 0x1F)) as u32 // SRAI
                        } else {
                            src >> (imm & 0x1F) // SRLI
                        }
                    }
                    _ => unreachable!(),
                };
                self.write_reg(inst.rd(), res);
            }

            // R-Type
            0x33 => {
                let s1 = self.read_reg(inst.rs1());
                let s2 = self.read_reg(inst.rs2());
                let res = match (inst.funct3(), inst.funct7()) {
                    (0x0, 0x00) => s1.wrapping_add(s2), // ADD
                    (0x0, 0x20) => s1.wrapping_sub(s2), // SUB
                    (0x1, 0x00) => s1 << (s2 & 0x1F),   // SLL
                    (0x2, 0x00) => {
                        if (s1 as i32) < (s2 as i32) {
                            1
                        } else {
                            0
                        }
                    } // SLT
                    (0x3, 0x00) => {
                        if s1 < s2 {
                            1
                        } else {
                            0
                        }
                    } // SLTU
                    (0x4, 0x00) => s1 ^ s2,             // XOR
                    (0x5, 0x00) => s1 >> (s2 & 0x1F),   // SRL
                    (0x5, 0x20) => ((s1 as i32) >> (s2 & 0x1F)) as u32, // SRA
                    (0x6, 0x00) => s1 | s2,             // OR
                    (0x7, 0x00) => s1 & s2,             // AND
                    _ => unimplemented!("Unsupported R-type combo: {inst:?}"),
                };
                self.write_reg(inst.rd(), res);
            }

            

            // --- JAL (Jump and Link) ---
            0x6F => {
                self.write_reg(inst.rd(), self.pc.wrapping_add(4));
                next_pc = self.pc.wrapping_add(inst.imm_j());
            }

            // --- JALR (Jump and Link Register) ---
            0x67 => {
                let target = self.read_reg(inst.rs1()).wrapping_add(inst.imm_i()) & !1;
                self.write_reg(inst.rd(), self.pc.wrapping_add(4));
                next_pc = target;
            }

            // --- BRANCH (BEQ, BNE, BLT, BGE, BLTU, BGEU) ---
            0x63 => {
                let s1 = self.read_reg(inst.rs1());
                let s2 = self.read_reg(inst.rs2());
                let taken = match inst.funct3() {
                    0x0 => s1 == s2,                   // BEQ
                    0x1 => s1 != s2,                   // BNE
                    0x4 => (s1 as i32) < (s2 as i32),  // BLT
                    0x5 => (s1 as i32) >= (s2 as i32), // BGE
                    0x6 => s1 < s2,                    // BLTU
                    0x7 => s1 >= s2,                   // BGEU
                    _ => unreachable!(),
                };

                if taken {
                    next_pc = self.pc.wrapping_add(inst.imm_b());
                }
            }

            // --- LOAD (LB, LH, LW, LBU, LHU) ---
            0x03 => {
                let addr = self.read_reg(inst.rs1()).wrapping_add(inst.imm_i());
                let val = match inst.funct3() {
                    0x0 => (self.bus.read_u8(addr) as i8 as i32) as u32, // LB
                    0x1 => (self.bus.read_u16(addr) as i16 as i32) as u32, // LH
                    0x2 => self.bus.read_u32(addr),                      // LW
                    0x4 => self.bus.read_u8(addr) as u32,                // LBU
                    0x5 => self.bus.read_u16(addr) as u32,               // LHU
                    _ => unimplemented!("Unsupported LOAD funct3"),
                };
                self.write_reg(inst.rd(), val);
            }

            // --- STORE (SB, SH, SW) ---
            0x23 => {
                let addr = self.read_reg(inst.rs1()).wrapping_add(inst.imm_s());
                let val = self.read_reg(inst.rs2());
                match inst.funct3() {
                    0x0 => self.bus.write_u8(addr, val as u8),
                    0x1 => self.bus.write_u16(addr, val as u16),
                    0x2 => self.bus.write_u32(addr, val),
                    _ => unimplemented!("Unsupported STORE funct3"),
                }
            }

            _ => unimplemented!("Opcode {:#x} not implemented", inst.opcode()),
        }

        self.pc = next_pc;
    }
}
