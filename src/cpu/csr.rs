// src/cpu/csr.rs
use crate::cpu::pmp::Pmp;

pub const MSTATUS: u16 = 0x300;
pub const MISA: u16    = 0x301;
pub const MIE: u16     = 0x304;
pub const MTVEC: u16   = 0x305;
pub const MEPC: u16    = 0x341;
pub const MCAUSE: u16  = 0x342;
pub const MTVAL: u16   = 0x343;
pub const MIP: u16     = 0x344;
pub const MCYCLE: u16  = 0xB00;

// PMP Configuration CSRs (RV32)
pub const CSR_PMPCFG0: u16 = 0x3A0;
pub const CSR_PMPCFG1: u16 = 0x3A1;
pub const CSR_PMPCFG2: u16 = 0x3A2;
pub const CSR_PMPCFG3: u16 = 0x3A3;

// PMP Address CSRs (0..15)
pub const CSR_PMPADDR0: u16  = 0x3B0;
pub const CSR_PMPADDR15: u16 = 0x3BF;

#[derive(Default)]
pub struct Csr {
    pub mstatus: u32,
    pub mie: u32,
    pub mtvec: u32,
    pub mepc: u32,
    pub mcause: u32,
    pub mtval: u32,
    pub mip: u32,
    pub cycle: u64,
    pub pmp: Pmp,
    pub priv_mode: u8, // 3 = M-Mode, 0 = U-Mode
}

impl Csr {
    pub fn new() -> Self {
        Self {
            mstatus: 0,
            mie: 0,
            mtvec: 0,
            mepc: 0,
            mcause: 0,
            mtval: 0,
            mip: 0,
            cycle: 0,
            pmp: Pmp::new(),
            priv_mode: 3, // Start in Machine mode
        }
    }

    pub fn read(&self, csr_addr: u16) -> u32 {
        match csr_addr {
            MSTATUS => self.mstatus,
            // RV32IMAC: MXL=1 (bit 30), A (bit 0), C (bit 2), I (bit 8), M (bit 12)
            MISA => (1 << 30) | (1 << 0) | (1 << 2) | (1 << 8) | (1 << 12),
            MIE => self.mie,
            MTVEC => self.mtvec,
            MEPC => self.mepc,
            MCAUSE => self.mcause,
            MTVAL => self.mtval,
            MIP => self.mip,
            MCYCLE | 0xC00 => (self.cycle & 0xFFFF_FFFF) as u32,
            0xC80 | 0xB80 => (self.cycle >> 32) as u32,

            // PMP Config CSRs (0x3A0..=0x3A3)
            CSR_PMPCFG0..=CSR_PMPCFG3 => {
                let idx = (csr_addr - CSR_PMPCFG0) as usize;
                self.pmp.read_cfg(idx)
            }

            // PMP Address CSRs (0x3B0..=0x3BF)
            CSR_PMPADDR0..=CSR_PMPADDR15 => {
                let idx = (csr_addr - CSR_PMPADDR0) as usize;
                self.pmp.read_addr(idx)
            }

            _ => 0, // Unimplemented CSRs default to 0
        }
    }

    pub fn write(&mut self, csr_addr: u16, val: u32) {
        match csr_addr {
            MSTATUS => self.mstatus = val,
            MIE => self.mie = val,
            MTVEC => self.mtvec = val,
            MEPC => self.mepc = val,
            MCAUSE => self.mcause = val,
            MTVAL => self.mtval = val,
            MIP => self.mip = val,

            // PMP Config CSRs (0x3A0..=0x3A3)
            CSR_PMPCFG0..=CSR_PMPCFG3 => {
                let idx = (csr_addr - CSR_PMPCFG0) as usize;
                self.pmp.write_cfg(idx, val);
            }

            // PMP Address CSRs (0x3B0..=0x3BF)
            CSR_PMPADDR0..=CSR_PMPADDR15 => {
                let idx = (csr_addr - CSR_PMPADDR0) as usize;
                self.pmp.write_addr(idx, val);
            }

            _ => {}
        }
    }
}