
#[derive(Debug, Clone, Copy)]
pub struct Instruction(pub u32);

impl Instruction {
    #[inline(always)]
    pub fn opcode(&self) -> u32 { self.0 & 0x7F }
    
    #[inline(always)]
    pub fn rd(&self) -> usize { ((self.0 >> 7) & 0x1F) as usize }
    
    #[inline(always)]
    pub fn funct3(&self) -> u32 { (self.0 >> 12) & 0x07 }
    
    #[inline(always)]
    pub fn rs1(&self) -> usize { ((self.0 >> 15) & 0x1F) as usize }
    
    #[inline(always)]
    pub fn rs2(&self) -> usize { ((self.0 >> 20) & 0x1F) as usize }
    
    #[inline(always)]
    pub fn funct7(&self) -> u32 { (self.0 >> 25) & 0x7F }

    // I-Type Immediate (Sign-Extended 12-bit)
    pub fn imm_i(&self) -> u32 {
        (self.0 as i32 >> 20) as u32
    }

    // S-Type Immediate (Store: SW, SH, SB)
    pub fn imm_s(&self) -> u32 {
        let imm4_0 = (self.0 >> 7) & 0x1F;
        let imm11_5 = ((self.0 as i32 >> 25) as u32) << 5;
        imm11_5 | imm4_0
    }

    // B-Type Immediate (Branch: BEQ, BNE, BLT, BGE...)
    pub fn imm_b(&self) -> u32 {
        let imm12 = ((self.0 as i32 >> 31) as u32) << 12;
        let imm11 = ((self.0 >> 7) & 0x01) << 11;
        let imm10_5 = ((self.0 >> 25) & 0x3F) << 5;
        let imm4_1 = ((self.0 >> 8) & 0x0F) << 1;
        imm12 | imm11 | imm10_5 | imm4_1
    }

    // U-Type Immediate (LUI, AUIPC)
    pub fn imm_u(&self) -> u32 {
        self.0 & 0xFFFF_F000
    }

    // J-Type Immediate (JAL)
    pub fn imm_j(&self) -> u32 {
        let imm20 = ((self.0 as i32 >> 31) as u32) << 20;
        let imm19_12 = (self.0 >> 12) & 0xFF;
        let imm11 = ((self.0 >> 20) & 0x01) << 11;
        let imm10_1 = ((self.0 >> 21) & 0x3FF) << 1;
        imm20 | (imm19_12 << 12) | imm11 | imm10_1
    }
}