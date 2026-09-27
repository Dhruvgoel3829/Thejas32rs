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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_csr_new() {
        let csr = Csr::new();
        assert_eq!(csr.mstatus, 0);
        assert_eq!(csr.mie, 0);
        assert_eq!(csr.mtvec, 0);
        assert_eq!(csr.mepc, 0);
        assert_eq!(csr.mcause, 0);
        assert_eq!(csr.mtval, 0);
        assert_eq!(csr.mip, 0);
        assert_eq!(csr.cycle, 0);
        assert_eq!(csr.priv_mode, 3);
    }

    #[test]
    fn test_csr_read_write_basic() {
        let mut csr = Csr::new();
        
        // Test MSTATUS
        csr.write(MSTATUS, 0x1234_5678);
        assert_eq!(csr.read(MSTATUS), 0x1234_5678);
        
        // Test MIE
        csr.write(MIE, 0x0000_0880); // MEIE + MTIE
        assert_eq!(csr.read(MIE), 0x0000_0880);
        
        // Test MTVEC
        csr.write(MTVEC, 0x8000_0100);
        assert_eq!(csr.read(MTVEC), 0x8000_0100);
        
        // Test MEPC
        csr.write(MEPC, 0x0000_1000);
        assert_eq!(csr.read(MEPC), 0x0000_1000);
        
        // Test MCAUSE
        csr.write(MCAUSE, 0x8000_000B);
        assert_eq!(csr.read(MCAUSE), 0x8000_000B);
        
        // Test MTVAL
        csr.write(MTVAL, 0xDEAD_BEEF);
        assert_eq!(csr.read(MTVAL), 0xDEAD_BEEF);
        
        // Test MIP
        csr.write(MIP, 0x0000_0880);
        assert_eq!(csr.read(MIP), 0x0000_0880);
    }

    #[test]
    fn test_csr_misa() {
        let csr = Csr::new();
        // RV32IMAC: MXL=1 (bit 30 for RV32), A (bit 0), C (bit 2), I (bit 8), M (bit 12)
        let expected = (1 << 30) | (1 << 0) | (1 << 2) | (1 << 8) | (1 << 12);
        assert_eq!(csr.read(MISA), expected);
    }

    #[test]
    fn test_csr_cycle() {
        let mut csr = Csr::new();
        csr.cycle = 0x1234_5678_9ABC_DEF0;
        
        // MCYCLE (lower 32 bits)
        assert_eq!(csr.read(MCYCLE), 0x9ABC_DEF0);
        
        // MCYCLEH (upper 32 bits) - 0xC80 or 0xB80
        assert_eq!(csr.read(0xC80), 0x1234_5678);
        assert_eq!(csr.read(0xB80), 0x1234_5678);
    }

    #[test]
    fn test_csr_pmp_cfg() {
        let mut csr = Csr::new();
        
        // Write PMPCFG0
        csr.write(CSR_PMPCFG0, 0x1F); // R=1, W=1, X=1, A=1 (NAPOT), L=0
        assert_eq!(csr.read(CSR_PMPCFG0), 0x1F);
        
        // Write PMPCFG1
        csr.write(CSR_PMPCFG1, 0x0F); // R=1, W=1, X=1, A=0 (TOR), L=0
        assert_eq!(csr.read(CSR_PMPCFG1), 0x0F);
        
        // Write PMPCFG2
        csr.write(CSR_PMPCFG2, 0x80); // L=1 (locked)
        assert_eq!(csr.read(CSR_PMPCFG2), 0x80);
        
        // Write PMPCFG3
        csr.write(CSR_PMPCFG3, 0x00); // OFF
        assert_eq!(csr.read(CSR_PMPCFG3), 0x00);
    }

    #[test]
    fn test_csr_pmp_addr() {
        let mut csr = Csr::new();
        
        // Write PMPADDR0
        csr.write(CSR_PMPADDR0, 0x0000_1000);
        assert_eq!(csr.read(CSR_PMPADDR0), 0x0000_1000);
        
        // Write PMPADDR15
        csr.write(CSR_PMPADDR15, 0x8000_0000);
        assert_eq!(csr.read(CSR_PMPADDR15), 0x8000_0000);
    }

    #[test]
    fn test_csr_unknown_returns_zero() {
        let csr = Csr::new();
        assert_eq!(csr.read(0x999), 0); // Unknown CSR
    }

    #[test]
    fn test_csr_write_unknown_ignored() {
        let mut csr = Csr::new();
        csr.write(0x999, 0x1234_5678); // Should not panic
        // No way to verify directly, but shouldn't crash
    }
}