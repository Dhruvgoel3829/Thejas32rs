// src/cpu/mod.rs
pub mod c_decode;
pub mod csr;
pub mod decode;

use crate::bus::Bus;
use c_decode::decompress;
use csr::Csr;
use decode::Instruction;

pub struct Cpu {
    pub regs: [u32; 32],
    pub pc: u32,
    pub csr: Csr,
    pub bus: Bus,
    pub atomic_reservation: Option<u32>, // Holds reserved physical memory address
}

const REG_NAMES: [&str; 32] = [
    "zero", "ra", "sp", "gp", "tp", "t0", "t1", "t2", "s0", "s1", "a0", "a1", "a2", "a3", "a4",
    "a5", "a6", "a7", "s2", "s3", "s4", "s5", "s6", "s7", "s8", "s9", "s10", "s11", "t3", "t4",
    "t5", "t6",
];

impl Cpu {
    pub fn new(bus: Bus, entry_point: u32) -> Self {
        Self {
            regs: [0; 32],
            pc: entry_point,
            csr: Csr::new(),
            bus,
            atomic_reservation: None,
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

    // In src/cpu/mod.rs
    pub fn dump_registers(&self) {
        eprintln!("\r\n=== Register State (PC: 0x{:08x}) ===", self.pc);
        for i in 0..32 {
            eprint!("{:4}: 0x{:08x} ", REG_NAMES[i], self.regs[i]);
            if (i + 1) % 4 == 0 {
                eprint!("\r\n");
            }
        }
        eprintln!(
            "mstatus: 0x{:08x}  mepc: 0x{:08x}  mcause: 0x{:08x}\r",
            self.csr.mstatus, self.csr.mepc, self.csr.mcause
        );
        eprintln!("=================================================\r");
    }

    fn check_interrupts(&mut self) -> Option<u32> {
        // 1. Update Timer Pending bit in MIP (bit 7: MTIP)
        if self.bus.clint.is_interrupt_pending() {
            self.csr.mip |= 1 << 7;
        } else {
            self.csr.mip &= !(1 << 7);
        }

        // 2. Update External Pending bit in MIP (bit 11: MEIP)
        if self.bus.plic.is_interrupt_pending() {
            self.csr.mip |= 1 << 11;
        } else {
            self.csr.mip &= !(1 << 11);
        }

        // Global interrupt enable check (mstatus.MIE, bit 3)
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

        // Save MIE (bit 3) into MPIE (bit 7)
        let mie = (self.csr.mstatus >> 3) & 1;
        self.csr.mstatus = (self.csr.mstatus & !(1 << 7)) | (mie << 7);

        // Clear MIE to disable nested interrupts
        self.csr.mstatus &= !(1 << 3);

        let mode = self.csr.mtvec & 0x3;
        let base = self.csr.mtvec & !0x3;
        let is_interrupt = (cause & 0x8000_0000) != 0;

        // Support both Direct (mode 0) and Vectored (mode 1)
        if mode == 1 && is_interrupt {
            let cause_num = cause & 0x7FFF_FFFF;
            self.pc = base.wrapping_add(cause_num * 4);
        } else {
            self.pc = base;
        }
        self.atomic_reservation = None;
    }

    pub fn step(&mut self, trace: bool) {
        // Check if UART received a byte and assert line 1 on the PLIC
        // UART RX line -> PLIC Source 1
        if (self.bus.uart.read_u8(0x05) & 0x01) != 0 {
            if self.bus.plic.claimed != 1 {
                self.bus.plic.set_interrupt_pending(1);
            }
        } else {
            self.bus.plic.pending &= !(1 << 1);
        }

        // Peripheral Timer -> PLIC Source 2
        if self.bus.timer.tick() {
            self.bus.plic.set_interrupt_pending(2);
        }

        // 1. Tick hardware timers and cycle counter
        self.bus.clint.tick();
        self.csr.cycle = self.csr.cycle.wrapping_add(1);

        // 2. Evaluate interrupt conditions
        if let Some(cause) = self.check_interrupts() {
            if trace {
                eprintln!("[TRAP] Cause: 0x{:08x} @ PC: 0x{:08x}", cause, self.pc);
            }
            self.handle_trap(cause, self.pc);
            return;
        }
        // 3. Fetch instruction (Variable-length 16-bit / 32-bit)
        let raw_16_lo = self.bus.read_u16(self.pc);
        let (raw, inst_len) = if (raw_16_lo & 0x3) != 0x3 {
            // 16-bit Compressed Instruction
            let decompressed = decompress(raw_16_lo).unwrap_or_else(|| {
                self.dump_registers();
                panic!(
                    "Illegal or unimplemented RV32C instruction 0x{:04x} at PC 0x{:08x}",
                    raw_16_lo, self.pc
                );
            });
            (decompressed, 2)
        } else {
            // 32-bit Instruction (composed from two 16-bit halfwords)
            let raw_16_hi = self.bus.read_u16(self.pc.wrapping_add(2));
            let full_inst = (raw_16_lo as u32) | ((raw_16_hi as u32) << 16);
            (full_inst, 4)
        };

        let inst = Instruction(raw);

        // 4. Trace logging if enabled
        // In src/cpu/mod.rs (inside step())
        if trace {
            if inst_len == 2 {
                eprint!(
                    "[0x{:08x}] (16b: {:04x} -> {:08x})  {:<8} rd:x{:02} rs1:x{:02} rs2:x{:02}\r\n",
                    self.pc,
                    raw_16_lo,
                    raw,
                    inst.mnemonic(),
                    inst.rd(),
                    inst.rs1(),
                    inst.rs2()
                );
            } else {
                eprint!(
                    "[0x{:08x}] {:08x}  {:<8} rd:x{:02} rs1:x{:02} rs2:x{:02}\r\n",
                    self.pc,
                    raw,
                    inst.mnemonic(),
                    inst.rd(),
                    inst.rs1(),
                    inst.rs2()
                );
            }
        }

        let mut next_pc = self.pc.wrapping_add(inst_len);
        // 5. Decode & Execute
        match inst.opcode() {
            // --- FENCE / FENCE.I ---
            0x0F => {}

            // --- SYSTEM / CSR (0x73) ---
            0x73 => {
                let funct3 = inst.funct3();
                let csr_addr = (raw >> 20) as u16;

                match funct3 {
                    // System calls and returns (funct3 == 0)
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
                                // MRET: Return from machine mode trap
                                let mpie = (self.csr.mstatus >> 7) & 1;
                                self.csr.mstatus = (self.csr.mstatus & !(1 << 3)) | (mpie << 3);
                                self.csr.mstatus |= 1 << 7; // Set MPIE = 1
                                next_pc = self.csr.mepc;
                            }
                            0x105 => {
                                // WFI (Wait For Interrupt)
                            }
                            _ => unimplemented!("Unsupported system funct12: {:#x}", funct12),
                        }
                    }

                    // Register-based CSR operations
                    0x1 | 0x2 | 0x3 => {
                        let t = self.csr.read(csr_addr);
                        let src = self.read_reg(inst.rs1());

                        // Do not write if rs1 == 0 for CSRRS/CSRRC (pure read)
                        if funct3 == 0x1 || inst.rs1() != 0 {
                            let new_val = match funct3 {
                                0x1 => src,        // CSRRW
                                0x2 => t | src,    // CSRRS
                                0x3 => t & (!src), // CSRRC
                                _ => unreachable!(),
                            };
                            self.csr.write(csr_addr, new_val);
                        }
                        self.write_reg(inst.rd(), t);
                    }

                    // Immediate-based CSR operations
                    0x5 | 0x6 | 0x7 => {
                        let t = self.csr.read(csr_addr);
                        let zimm = inst.rs1() as u32;

                        // Do not write if zimm == 0 for CSRRSI/CSRRCI (pure read)
                        if funct3 == 0x5 || zimm != 0 {
                            let new_val = match funct3 {
                                0x5 => zimm,        // CSRRWI
                                0x6 => t | zimm,    // CSRRSI
                                0x7 => t & (!zimm), // CSRRCI
                                _ => unreachable!(),
                            };
                            self.csr.write(csr_addr, new_val);
                        }
                        self.write_reg(inst.rd(), t);
                    }

                    _ => unimplemented!("Unsupported System funct3: {:#x}", funct3),
                }
            }

            // --- LUI (Load Upper Immediate) ---
            0x37 => {
                self.write_reg(inst.rd(), inst.imm_u());
            }

            // --- AUIPC (Add Upper Immediate to PC) ---
            0x17 => {
                let val = self.pc.wrapping_add(inst.imm_u());
                self.write_reg(inst.rd(), val);
            }

            // --- OP-IMM (ADDI, SLTI, SLTIU, XORI, ORI, ANDI, SLLI, SRLI, SRAI) ---
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

            // --- R-Type & RV32M Extension ---
            0x33 => {
                let s1 = self.read_reg(inst.rs1());
                let s2 = self.read_reg(inst.rs2());
                let res = match (inst.funct7(), inst.funct3()) {
                    // Base RV32I
                    (0x00, 0x0) => s1.wrapping_add(s2), // ADD
                    (0x20, 0x0) => s1.wrapping_sub(s2), // SUB
                    (0x00, 0x1) => s1 << (s2 & 0x1F),   // SLL
                    (0x00, 0x2) => {
                        if (s1 as i32) < (s2 as i32) {
                            1
                        } else {
                            0
                        }
                    } // SLT
                    (0x00, 0x3) => {
                        if s1 < s2 {
                            1
                        } else {
                            0
                        }
                    } // SLTU
                    (0x00, 0x4) => s1 ^ s2,             // XOR
                    (0x00, 0x5) => s1 >> (s2 & 0x1F),   // SRL
                    (0x20, 0x5) => ((s1 as i32) >> (s2 & 0x1F)) as u32, // SRA
                    (0x00, 0x6) => s1 | s2,             // OR
                    (0x00, 0x7) => s1 & s2,             // AND

                    // RV32M Extension (funct7 == 0x01)
                    (0x01, 0x0) => s1.wrapping_mul(s2), // MUL
                    (0x01, 0x1) => ((s1 as i32 as i64).wrapping_mul(s2 as i32 as i64) >> 32) as u32, // MULH
                    (0x01, 0x2) => ((s1 as i32 as i64).wrapping_mul(s2 as u64 as i64) >> 32) as u32, // MULHSU
                    (0x01, 0x3) => ((s1 as u64).wrapping_mul(s2 as u64) >> 32) as u32, // MULHU
                    (0x01, 0x4) => {
                        // DIV
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
                        // DIVU
                        if s2 == 0 { 0xFFFF_FFFF } else { s1 / s2 }
                    }
                    (0x01, 0x6) => {
                        // REM
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
                        // REMU
                        if s2 == 0 { s1 } else { s1 % s2 }
                    }
                    _ => unimplemented!("Unsupported R/M type: {inst:?}"),
                };
                self.write_reg(inst.rd(), res);
            }

            // --- JAL (Jump and Link) ---
            0x6F => {
                self.write_reg(inst.rd(), self.pc.wrapping_add(inst_len));
                next_pc = self.pc.wrapping_add(inst.imm_j());
            }

            // --- JALR (Jump and Link Register) ---
            0x67 => {
                let target = self.read_reg(inst.rs1()).wrapping_add(inst.imm_i()) & !1;
                self.write_reg(inst.rd(), self.pc.wrapping_add(inst_len));
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
                    _ => unimplemented!("Unsupported LOAD funct3: {:#x}", inst.funct3()),
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
                    _ => unimplemented!("Unsupported STORE funct3: {:#x}", inst.funct3()),
                }
                self.atomic_reservation = None;
            }

            // --- RV32A Atomic Extension (Opcode 0x2F) ---
            0x2F => {
                let addr = self.read_reg(inst.rs1());
                let src2 = self.read_reg(inst.rs2());
                let funct5 = inst.funct5();

                match funct5 {
                    // LR.W (Load-Reserved Word)
                    0b00010 => {
                        let val = self.bus.read_u32(addr);
                        self.atomic_reservation = Some(addr);
                        self.write_reg(inst.rd(), val);
                    }

                    // SC.W (Store-Conditional Word)
                    0b00011 => {
                        if self.atomic_reservation == Some(addr) {
                            self.bus.write_u32(addr, src2);
                            self.write_reg(inst.rd(), 0); // 0 = Success
                            self.atomic_reservation = None;
                        } else {
                            self.write_reg(inst.rd(), 1); // 1 = Failure
                        }
                    }

                    // AMOSWAP.W
                    0b00001 => {
                        let orig = self.bus.read_u32(addr);
                        self.bus.write_u32(addr, src2);
                        self.write_reg(inst.rd(), orig);
                    }

                    // AMOADD.W
                    0b00000 => {
                        let orig = self.bus.read_u32(addr);
                        let res = orig.wrapping_add(src2);
                        self.bus.write_u32(addr, res);
                        self.write_reg(inst.rd(), orig);
                    }

                    // AMOXOR.W
                    0b00100 => {
                        let orig = self.bus.read_u32(addr);
                        let res = orig ^ src2;
                        self.bus.write_u32(addr, res);
                        self.write_reg(inst.rd(), orig);
                    }

                    // AMOAND.W
                    0b01100 => {
                        let orig = self.bus.read_u32(addr);
                        let res = orig & src2;
                        self.bus.write_u32(addr, res);
                        self.write_reg(inst.rd(), orig);
                    }

                    // AMOOR.W
                    0b01000 => {
                        let orig = self.bus.read_u32(addr);
                        let res = orig | src2;
                        self.bus.write_u32(addr, res);
                        self.write_reg(inst.rd(), orig);
                    }

                    // AMOMIN.W (Signed Minimum)
                    0b10000 => {
                        let orig = self.bus.read_u32(addr);
                        let res = if (orig as i32) < (src2 as i32) {
                            orig
                        } else {
                            src2
                        };
                        self.bus.write_u32(addr, res);
                        self.write_reg(inst.rd(), orig);
                    }

                    // AMOMAX.W (Signed Maximum)
                    0b10100 => {
                        let orig = self.bus.read_u32(addr);
                        let res = if (orig as i32) > (src2 as i32) {
                            orig
                        } else {
                            src2
                        };
                        self.bus.write_u32(addr, res);
                        self.write_reg(inst.rd(), orig);
                    }

                    // AMOMINU.W (Unsigned Minimum)
                    0b11000 => {
                        let orig = self.bus.read_u32(addr);
                        let res = if orig < src2 { orig } else { src2 };
                        self.bus.write_u32(addr, res);
                        self.write_reg(inst.rd(), orig);
                    }

                    // AMOMAXU.W (Unsigned Maximum)
                    0b11100 => {
                        let orig = self.bus.read_u32(addr);
                        let res = if orig > src2 { orig } else { src2 };
                        self.bus.write_u32(addr, res);
                        self.write_reg(inst.rd(), orig);
                    }

                    _ => unimplemented!(
                        "Unsupported RV32A funct5: {:#b} at PC {:#x}",
                        funct5,
                        self.pc
                    ),
                }
            }

            _ => unimplemented!(
                "Opcode {:#x} not implemented at PC {:#x}",
                inst.opcode(),
                self.pc
            ),
        }

        self.pc = next_pc;
    }
}
