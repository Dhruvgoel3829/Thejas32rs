pub const MSTATUS: u16 = 0x300;
pub const MISA: u16    = 0x301;
pub const MIE: u16     = 0x304;
pub const MTVEC: u16   = 0x305;
pub const MEPC: u16    = 0x341;
pub const MCAUSE: u16  = 0x342;
pub const MTVAL: u16   = 0x343;
pub const MIP: u16     = 0x344;
pub const MCYCLE: u16  = 0xB00;

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
}

impl Csr {
    pub fn new() -> Self {
        Self {
            // MISA: 32-bit (MXL=1), RV32IM supported (Bit 8 for 'I', Bit 12 for 'M')
            mstatus: 0,
            mie: 0,
            mtvec: 0,
            mepc: 0,
            mcause: 0,
            mtval: 0,
            mip: 0,
            cycle: 0,
        }
    }

    pub fn read(&self, csr_addr: u16) -> u32 {
        match csr_addr {
            MSTATUS => self.mstatus,
            MISA    => (1 << 30) | (1 << 8) | (1 << 12), // RV32IM signature
            MIE     => self.mie,
            MTVEC   => self.mtvec,
            MEPC    => self.mepc,
            MCAUSE  => self.mcause,
            MTVAL   => self.mtval,
            MIP     => self.mip,
            MCYCLE  => self.cycle as u32,
            _ => 0, // Unimplemented CSRs default to 0
        }
    }

    pub fn write(&mut self, csr_addr: u16, val: u32) {
        match csr_addr {
            MSTATUS => self.mstatus = val,
            MIE     => self.mie = val,
            MTVEC   => self.mtvec = val,
            MEPC    => self.mepc = val,
            MCAUSE  => self.mcause = val,
            MTVAL   => self.mtval = val,
            MIP     => self.mip = val,
            _ => {}
        }
    }
}