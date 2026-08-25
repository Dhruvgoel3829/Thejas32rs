// src/cpu/mod.rs
pub mod csr;
pub mod decode;

use crate::bus::Bus;
use csr::Csr;
use decode::Instruction;

pub struct Cpu {
    pub regs: [u32; 32],
    pub pc: u32,
    pub csr: Csr,
    pub bus: Bus,
}

impl Cpu {
    pub fn new(bus: Bus, entry_point: u32) -> Self {
        Self {
            regs: [0; 32],
            pc: entry_point,
            csr: Csr::new(),
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

    // src/cpu/mod.rs (inside check_interrupts)
    fn check_interrupts(&mut self) -> Option<u32> {
        // 1. Update Timer Pending (MTIP, bit 7)
        if self.bus.clint.is_interrupt_pending() {
            self.csr.mip |= 1 << 7;
        } else {
            self.csr.mip &= !(1 << 7);
        }

        // 2. Update External Pending (MEIP, bit 11)
        if self.bus.plic.is_interrupt_pending() {
            self.csr.mip |= 1 << 11;
        } else {
            self.csr.mip &= !(1 << 11);
        }

        let mstatus_mie = (self.csr.mstatus & (1 << 3)) != 0;
        if !mstatus_mie {
            return None;
        }

        let mie_meie = (self.csr.mie & (1 << 11)) != 0;
        let mip_meip = (self.csr.mip & (1 << 11)) != 0;

        // External interrupts have higher priority than timer interrupts
        if mie_meie && mip_meip {
            return Some(0x8000_000B); // Machine External Interrupt cause
        }

        let mie_mtie = (self.csr.mie & (1 << 7)) != 0;
        let mip_mtip = (self.csr.mip & (1 << 7)) != 0;

        if mie_mtie && mip_mtip {
            return Some(0x8000_0007); // Machine Timer Interrupt cause
        }

        None
    }

    fn handle_trap(&mut self, cause: u32, epc: u32) {
        self.csr.mepc = epc;
        self.csr.mcause = cause;

        // Save MIE to MPIE (bit 3 -> bit 7)
        let mie = (self.csr.mstatus >> 3) & 1;
        self.csr.mstatus = (self.csr.mstatus & !(1 << 7)) | (mie << 7);

        // Clear MIE (disable interrupts)
        self.csr.mstatus &= !(1 << 3);

        // Jump to mtvec (handling direct mode)
        let base_vector = self.csr.mtvec & !0x3;
        self.pc = base_vector;
    }

    pub fn step(&mut self) {
        // 1. Tick timers
        self.bus.clint.tick();
        self.csr.cycle += 1;

        // 2. Check for pending interrupts before executing
        if let Some(cause) = self.check_interrupts() {
            self.handle_trap(cause, self.pc);
            return;
        }

        // 3. Fetch
        let raw = self.bus.read_u32(self.pc);
        let inst = Instruction(raw);
        let mut next_pc = self.pc.wrapping_add(4);

        match inst.opcode() {
            0x0F => {} // FENCE

            0x73 => {
                // SYSTEM / CSR
                let funct3 = inst.funct3();
                let csr_addr = (raw >> 20) as u16;

                match funct3 {
                    0x0 => {
                        let funct12 = (raw >> 20) & 0xFFF;
                        match funct12 {
                            0x000 => {
                                // ECALL
                                self.handle_trap(11, self.pc);
                                return;
                            }
                            0x001 => {
                                // EBREAK
                                self.handle_trap(3, self.pc);
                                return;
                            }
                            0x302 => {
                                // MRET
                                // Restore MIE from MPIE
                                let mpie = (self.csr.mstatus >> 7) & 1;
                                self.csr.mstatus = (self.csr.mstatus & !(1 << 3)) | (mpie << 3);
                                self.csr.mstatus |= 1 << 7; // Set MPIE = 1
                                next_pc = self.csr.mepc;
                            }
                            0x105 => {} // WFI
                            _ => unimplemented!("System funct12: {:#x}", funct12),
                        }
                    }
                    0x1 | 0x2 | 0x3 => {
                        let t = self.csr.read(csr_addr);
                        let src = self.read_reg(inst.rs1());
                        let new_val = match funct3 {
                            0x1 => src,
                            0x2 => t | src,
                            0x3 => t & (!src),
                            _ => unreachable!(),
                        };
                        self.csr.write(csr_addr, new_val);
                        self.write_reg(inst.rd(), t);
                    }
                    0x5 | 0x6 | 0x7 => {
                        let t = self.csr.read(csr_addr);
                        let zimm = inst.rs1() as u32;
                        let new_val = match funct3 {
                            0x5 => zimm,
                            0x6 => t | zimm,
                            0x7 => t & (!zimm),
                            _ => unreachable!(),
                        };
                        self.csr.write(csr_addr, new_val);
                        self.write_reg(inst.rd(), t);
                    }
                    _ => unimplemented!("System funct3: {:#x}", funct3),
                }
            }

            // ... (keep LUI 0x37, AUIPC 0x17, OP-IMM 0x13, R-type/M 0x33, BRANCH 0x63, JAL 0x6F, JALR 0x67, LOAD 0x03, STORE 0x23)
            0x37 => self.write_reg(inst.rd(), inst.imm_u()),
            0x17 => {
                let val = self.pc.wrapping_add(inst.imm_u());
                self.write_reg(inst.rd(), val);
            }
            0x13 => {
                let src = self.read_reg(inst.rs1());
                let imm = inst.imm_i();
                let res = match inst.funct3() {
                    0x0 => src.wrapping_add(imm),
                    0x2 => {
                        if (src as i32) < (imm as i32) {
                            1
                        } else {
                            0
                        }
                    }
                    0x3 => {
                        if src < imm {
                            1
                        } else {
                            0
                        }
                    }
                    0x4 => src ^ imm,
                    0x6 => src | imm,
                    0x7 => src & imm,
                    0x1 => src << (imm & 0x1F),
                    0x5 => {
                        if (inst.funct7() & 0x20) != 0 {
                            ((src as i32) >> (imm & 0x1F)) as u32
                        } else {
                            src >> (imm & 0x1F)
                        }
                    }
                    _ => unreachable!(),
                };
                self.write_reg(inst.rd(), res);
            }
            0x33 => {
                let s1 = self.read_reg(inst.rs1());
                let s2 = self.read_reg(inst.rs2());
                let res = match (inst.funct7(), inst.funct3()) {
                    (0x00, 0x0) => s1.wrapping_add(s2),
                    (0x20, 0x0) => s1.wrapping_sub(s2),
                    (0x00, 0x1) => s1 << (s2 & 0x1F),
                    (0x00, 0x2) => {
                        if (s1 as i32) < (s2 as i32) {
                            1
                        } else {
                            0
                        }
                    }
                    (0x00, 0x3) => {
                        if s1 < s2 {
                            1
                        } else {
                            0
                        }
                    }
                    (0x00, 0x4) => s1 ^ s2,
                    (0x00, 0x5) => s1 >> (s2 & 0x1F),
                    (0x20, 0x5) => ((s1 as i32) >> (s2 & 0x1F)) as u32,
                    (0x00, 0x6) => s1 | s2,
                    (0x00, 0x7) => s1 & s2,
                    (0x01, 0x0) => s1.wrapping_mul(s2),
                    (0x01, 0x1) => ((s1 as i32 as i64).wrapping_mul(s2 as i32 as i64) >> 32) as u32,
                    (0x01, 0x2) => ((s1 as i32 as i64).wrapping_mul(s2 as u64 as i64) >> 32) as u32,
                    (0x01, 0x3) => ((s1 as u64).wrapping_mul(s2 as u64) >> 32) as u32,
                    (0x01, 0x4) => {
                        let a = s1 as i32;
                        let b = s2 as i32;
                        if b == 0 {
                            0xFFFF_FFFF
                        } else if a == i32::MIN && b == -1 {
                            i32::MIN as u32
                        } else {
                            (a / b) as u32
                        }
                    }
                    (0x01, 0x5) => {
                        if s2 == 0 {
                            0xFFFF_FFFF
                        } else {
                            s1 / s2
                        }
                    }
                    (0x01, 0x6) => {
                        let a = s1 as i32;
                        let b = s2 as i32;
                        if b == 0 {
                            s1
                        } else if a == i32::MIN && b == -1 {
                            0
                        } else {
                            (a % b) as u32
                        }
                    }
                    (0x01, 0x7) => {
                        if s2 == 0 {
                            s1
                        } else {
                            s1 % s2
                        }
                    }
                    _ => unimplemented!("Unsupported R/M type: {inst:?}"),
                };
                self.write_reg(inst.rd(), res);
            }
            0x6F => {
                self.write_reg(inst.rd(), self.pc.wrapping_add(4));
                next_pc = self.pc.wrapping_add(inst.imm_j());
            }
            0x67 => {
                let target = self.read_reg(inst.rs1()).wrapping_add(inst.imm_i()) & !1;
                self.write_reg(inst.rd(), self.pc.wrapping_add(4));
                next_pc = target;
            }
            0x63 => {
                let s1 = self.read_reg(inst.rs1());
                let s2 = self.read_reg(inst.rs2());
                let taken = match inst.funct3() {
                    0x0 => s1 == s2,
                    0x1 => s1 != s2,
                    0x4 => (s1 as i32) < (s2 as i32),
                    0x5 => (s1 as i32) >= (s2 as i32),
                    0x6 => s1 < s2,
                    0x7 => s1 >= s2,
                    _ => unreachable!(),
                };
                if taken {
                    next_pc = self.pc.wrapping_add(inst.imm_b());
                }
            }
            0x03 => {
                let addr = self.read_reg(inst.rs1()).wrapping_add(inst.imm_i());
                let val = match inst.funct3() {
                    0x0 => (self.bus.read_u8(addr) as i8 as i32) as u32,
                    0x1 => (self.bus.read_u16(addr) as i16 as i32) as u32,
                    0x2 => self.bus.read_u32(addr),
                    0x4 => self.bus.read_u8(addr) as u32,
                    0x5 => self.bus.read_u16(addr) as u32,
                    _ => unimplemented!("Unsupported LOAD funct3"),
                };
                self.write_reg(inst.rd(), val);
            }
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
