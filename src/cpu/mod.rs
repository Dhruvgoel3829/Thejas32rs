// src/cpu/mod.rs
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
                    0x0 => src.wrapping_add(imm),                         // ADDI
                    0x2 => if (src as i32) < (imm as i32) { 1 } else { 0 }, // SLTI
                    0x3 => if src < imm { 1 } else { 0 },                 // SLTIU
                    0x4 => src ^ imm,                                    // XORI
                    0x6 => src | imm,                                    // ORI
                    0x7 => src & imm,                                    // ANDI
                    0x1 => src << (imm & 0x1F),                          // SLLI
                    0x5 => {
                        if (inst.funct7() & 0x20) != 0 {
                            ((src as i32) >> (imm & 0x1F)) as u32        // SRAI
                        } else {
                            src >> (imm & 0x1F)                          // SRLI
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
                    (0x0, 0x00) => s1.wrapping_add(s2),              // ADD
                    (0x0, 0x20) => s1.wrapping_sub(s2),              // SUB
                    (0x1, 0x00) => s1 << (s2 & 0x1F),                // SLL
                    (0x2, 0x00) => if (s1 as i32) < (s2 as i32) { 1 } else { 0 }, // SLT
                    (0x3, 0x00) => if s1 < s2 { 1 } else { 0 },      // SLTU
                    (0x4, 0x00) => s1 ^ s2,                          // XOR
                    (0x5, 0x00) => s1 >> (s2 & 0x1F),                // SRL
                    (0x5, 0x20) => ((s1 as i32) >> (s2 & 0x1F)) as u32, // SRA
                    (0x6, 0x00) => s1 | s2,                          // OR
                    (0x7, 0x00) => s1 & s2,                          // AND
                    _ => unimplemented!("Unsupported R-type combo: {inst:?}"),
                };
                self.write_reg(inst.rd(), res);
            }

            _ => unimplemented!("Opcode {:#x} not implemented", inst.opcode()),
        }

        self.pc = next_pc;
    }
}