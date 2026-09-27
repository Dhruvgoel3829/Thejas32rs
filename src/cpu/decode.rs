#[derive(Debug, Clone, Copy)]
pub struct Instruction(pub u32);

impl Instruction {
    #[inline(always)]
    pub fn opcode(&self) -> u32 {
        self.0 & 0x7F
    }

    #[inline(always)]
    pub fn rd(&self) -> usize {
        ((self.0 >> 7) & 0x1F) as usize
    }

    #[inline(always)]
    pub fn funct3(&self) -> u32 {
        (self.0 >> 12) & 0x07
    }

    #[inline(always)]
    pub fn rs1(&self) -> usize {
        ((self.0 >> 15) & 0x1F) as usize
    }

    #[inline(always)]
    pub fn rs2(&self) -> usize {
        ((self.0 >> 20) & 0x1F) as usize
    }

    #[inline(always)]
    pub fn funct7(&self) -> u32 {
        (self.0 >> 25) & 0x7F
    }

    #[inline(always)]
    pub fn funct5(&self) -> u32 {
        (self.0 >> 27) & 0x1F
    }

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

    pub fn mnemonic(&self) -> &'static str {
        match self.opcode() {
            0x37 => "lui",
            0x17 => "auipc",
            0x6F => "jal",
            0x67 => "jalr",
            0x63 => match self.funct3() {
                0x0 => "beq",
                0x1 => "bne",
                0x4 => "blt",
                0x5 => "bge",
                0x6 => "bltu",
                0x7 => "bgeu",
                _ => "branch_unknown",
            },
            0x03 => match self.funct3() {
                0x0 => "lb",
                0x1 => "lh",
                0x2 => "lw",
                0x4 => "lbu",
                0x5 => "lhu",
                _ => "load_unknown",
            },
            0x23 => match self.funct3() {
                0x0 => "sb",
                0x1 => "sh",
                0x2 => "sw",
                _ => "store_unknown",
            },
            0x13 => match self.funct3() {
                0x0 => "addi",
                0x2 => "slti",
                0x3 => "sltiu",
                0x4 => "xori",
                0x6 => "ori",
                0x7 => "andi",
                0x1 => "slli",
                0x5 => {
                    if (self.funct7() & 0x20) != 0 {
                        "srai"
                    } else {
                        "srli"
                    }
                }
                _ => "op_imm_unknown",
            },
            0x33 => match (self.funct7(), self.funct3()) {
                (0x00, 0x0) => "add",
                (0x20, 0x0) => "sub",
                (0x00, 0x1) => "sll",
                (0x00, 0x2) => "slt",
                (0x00, 0x3) => "sltu",
                (0x00, 0x4) => "xor",
                (0x00, 0x5) => "srl",
                (0x20, 0x5) => "sra",
                (0x00, 0x6) => "or",
                (0x00, 0x7) => "and",
                (0x01, 0x0) => "mul",
                (0x01, 0x1) => "mulh",
                (0x01, 0x2) => "mulhsu",
                (0x01, 0x3) => "mulhu",
                (0x01, 0x4) => "div",
                (0x01, 0x5) => "divu",
                (0x01, 0x6) => "rem",
                (0x01, 0x7) => "remu",
                _ => "op_unknown",
            },
            0x0F => "fence",
            0x73 => match self.funct3() {
                0x0 => match (self.0 >> 20) & 0xFFF {
                    0x000 => "ecall",
                    0x001 => "ebreak",
                    0x302 => "mret",
                    0x105 => "wfi",
                    _ => "sys_unknown",
                },
                0x1 => "csrrw",
                0x2 => "csrrs",
                0x3 => "csrrc",
                0x5 => "csrrwi",
                0x6 => "csrrsi",
                0x7 => "csrrci",
                _ => "csr_unknown",
            },

            0x2F => match self.funct5() {
                0b00010 => "lr.w",
                0b00011 => "sc.w",
                0b00001 => "amoswap.w",
                0b00000 => "amoadd.w",
                0b00100 => "amoxor.w",
                0b01100 => "amoand.w",
                0b01000 => "amoor.w",
                0b10000 => "amomin.w",
                0b10100 => "amomax.w",
                0b11000 => "amominu.w",
                0b11100 => "amomaxu.w",
                _ => "amo_unknown",
            },
            _ => "unknown",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::Instruction;

    #[test]
    fn test_opcode_extraction() {
        let inst = Instruction(0x0000_0013); // addi x0, x0, 0
        assert_eq!(inst.opcode(), 0x13);
    }

    #[test]
    fn test_rd_extraction() {
        let inst = Instruction(0x0010_0093); // addi x1, x0, 1
        assert_eq!(inst.rd(), 1);
        let inst = Instruction(0x0020_8113); // addi x2, x1, 2
        assert_eq!(inst.rd(), 2);
        let inst = Instruction(0x0031_0193); // addi x3, x2, 3
        assert_eq!(inst.rd(), 3);
    }

    #[test]
    fn test_rs1_rs2_extraction() {
        let inst = Instruction(0x0020_8093); // addi x1, x1, 2
        assert_eq!(inst.rs1(), 1);
        let inst = Instruction(0x0030_8113); // addi x2, x1, 3 (wait, rs1=1)
        assert_eq!(inst.rs1(), 1);
        let inst = Instruction(0x0041_01B3); // add x3, x2, x4
        assert_eq!(inst.rs1(), 2);
        assert_eq!(inst.rs2(), 4);
    }

    #[test]
    fn test_funct3_extraction() {
        let inst = Instruction(0x0010_0093); // addi
        assert_eq!(inst.funct3(), 0x0);
        let inst = Instruction(0x0020_4093); // xori (funct3=4)
        assert_eq!(inst.funct3(), 0x4);
        let inst = Instruction(0x0030_6093); // ori (funct3=6)
        assert_eq!(inst.funct3(), 0x6);
    }

    #[test]
    fn test_funct7_extraction() {
        let inst = Instruction(0x0000_0033); // add
        assert_eq!(inst.funct7(), 0x00);
        let inst = Instruction(0x4000_0033); // sub
        assert_eq!(inst.funct7(), 0x20);
        let inst = Instruction(0x0200_0033); // mul
        assert_eq!(inst.funct7(), 0x01);
    }

    #[test]
    fn test_imm_i() {
        let inst = Instruction(0x0010_0093); // addi x1, x0, 1
        assert_eq!(inst.imm_i(), 1);
        let inst = Instruction(0xFFF0_0093); // addi x1, x0, -1
        assert_eq!(inst.imm_i(), 0xFFFF_FFFF); // sign-extended to 32 bits
        let inst = Instruction(0x8000_0093); // addi x1, x0, -2048
        assert_eq!(inst.imm_i(), 0xFFFF_F800); // sign-extended to 32 bits
    }

    #[test]
    fn test_imm_s() {
        let inst = Instruction(0x0000_2023); // sw x1, 0(x0)
        assert_eq!(inst.imm_s(), 0);
        let inst = Instruction(0x0021_2423); // sw x2, 8(x2)
        assert_eq!(inst.imm_s(), 8);
        let inst = Instruction(0xFE21_2C23); // sw x2, -8(x2)
        assert_eq!(inst.imm_s(), 0xFFFF_FFF8); // sign-extended -8
    }

#[test]
    fn test_imm_b() {
        let inst = Instruction(0x0000_0063); // beq x0, x0, 0
        assert_eq!(inst.imm_b(), 0);
        let inst = Instruction(0x0000_0263); // beq x0, x0, 4
        assert_eq!(inst.imm_b(), 4);
        let inst = Instruction(0xFE00_0FE3); // beq x0, x0, -4 (offset = -4, imm = -2)
        assert_eq!(inst.imm_b(), 0xFFFF_FFFE); // sign-extended -2 (immediate value, not offset)
    }

    #[test]
    fn test_imm_u() {
        let inst = Instruction(0x0000_0037); // lui x0, 0
        assert_eq!(inst.imm_u(), 0);
        let inst = Instruction(0x1234_5037); // lui x0, 0x12345
        assert_eq!(inst.imm_u(), 0x1234_5000);
    }

    #[test]
    fn test_imm_j() {
        let inst = Instruction(0x0000_006F); // jal x0, 0
        assert_eq!(inst.imm_j(), 0);
        let inst = Instruction(0x0040_006F); // jal x0, 4
        assert_eq!(inst.imm_j(), 4);
    }

    #[test]
    fn test_mnemonic_rv32i() {
        assert_eq!(Instruction(0x0000_0037).mnemonic(), "lui");
        assert_eq!(Instruction(0x0000_0017).mnemonic(), "auipc");
        assert_eq!(Instruction(0x0000_006F).mnemonic(), "jal");
        assert_eq!(Instruction(0x0000_0067).mnemonic(), "jalr");
        assert_eq!(Instruction(0x0000_0063).mnemonic(), "beq");
        assert_eq!(Instruction(0x0000_1063).mnemonic(), "bne");
        assert_eq!(Instruction(0x0000_4063).mnemonic(), "blt");
        assert_eq!(Instruction(0x0000_5063).mnemonic(), "bge");
        assert_eq!(Instruction(0x0000_6063).mnemonic(), "bltu");
        assert_eq!(Instruction(0x0000_7063).mnemonic(), "bgeu");
        assert_eq!(Instruction(0x0000_0003).mnemonic(), "lb");
        assert_eq!(Instruction(0x0000_1003).mnemonic(), "lh");
        assert_eq!(Instruction(0x0000_2003).mnemonic(), "lw");
        assert_eq!(Instruction(0x0000_4003).mnemonic(), "lbu");
        assert_eq!(Instruction(0x0000_5003).mnemonic(), "lhu");
        assert_eq!(Instruction(0x0000_0023).mnemonic(), "sb");
        assert_eq!(Instruction(0x0000_1023).mnemonic(), "sh");
        assert_eq!(Instruction(0x0000_2023).mnemonic(), "sw");
        assert_eq!(Instruction(0x0000_0013).mnemonic(), "addi");
        assert_eq!(Instruction(0x0000_2013).mnemonic(), "slti");
        assert_eq!(Instruction(0x0000_3013).mnemonic(), "sltiu");
        assert_eq!(Instruction(0x0000_4013).mnemonic(), "xori");
        assert_eq!(Instruction(0x0000_6013).mnemonic(), "ori");
        assert_eq!(Instruction(0x0000_7013).mnemonic(), "andi");
        assert_eq!(Instruction(0x0000_1013).mnemonic(), "slli");
        assert_eq!(Instruction(0x0000_5013).mnemonic(), "srli");
        assert_eq!(Instruction(0x4000_5013).mnemonic(), "srai");
        assert_eq!(Instruction(0x0000_0033).mnemonic(), "add");
        assert_eq!(Instruction(0x4000_0033).mnemonic(), "sub");
        assert_eq!(Instruction(0x0000_1033).mnemonic(), "sll");
        assert_eq!(Instruction(0x0000_2033).mnemonic(), "slt");
        assert_eq!(Instruction(0x0000_3033).mnemonic(), "sltu");
        assert_eq!(Instruction(0x0000_4033).mnemonic(), "xor");
        assert_eq!(Instruction(0x0000_5033).mnemonic(), "srl");
        assert_eq!(Instruction(0x4000_5033).mnemonic(), "sra");
        assert_eq!(Instruction(0x0000_6033).mnemonic(), "or");
        assert_eq!(Instruction(0x0000_7033).mnemonic(), "and");
    }

    #[test]
    fn test_mnemonic_rv32m() {
        assert_eq!(Instruction(0x0200_0033).mnemonic(), "mul");
        assert_eq!(Instruction(0x0200_1033).mnemonic(), "mulh");
        assert_eq!(Instruction(0x0200_2033).mnemonic(), "mulhsu");
        assert_eq!(Instruction(0x0200_3033).mnemonic(), "mulhu");
        assert_eq!(Instruction(0x0200_4033).mnemonic(), "div");
        assert_eq!(Instruction(0x0200_5033).mnemonic(), "divu");
        assert_eq!(Instruction(0x0200_6033).mnemonic(), "rem");
        assert_eq!(Instruction(0x0200_7033).mnemonic(), "remu");
    }

    #[test]
    fn test_mnemonic_system() {
        assert_eq!(Instruction(0x0000_0073).mnemonic(), "ecall");
        assert_eq!(Instruction(0x0010_0073).mnemonic(), "ebreak");
        assert_eq!(Instruction(0x3020_0073).mnemonic(), "mret");
        assert_eq!(Instruction(0x1050_0073).mnemonic(), "wfi");
        assert_eq!(Instruction(0x0000_1073).mnemonic(), "csrrw");
        assert_eq!(Instruction(0x0000_2073).mnemonic(), "csrrs");
        assert_eq!(Instruction(0x0000_3073).mnemonic(), "csrrc");
        assert_eq!(Instruction(0x0000_5073).mnemonic(), "csrrwi");
        assert_eq!(Instruction(0x0000_6073).mnemonic(), "csrrsi");
        assert_eq!(Instruction(0x0000_7073).mnemonic(), "csrrci");
    }

    #[test]
    fn test_mnemonic_rv32a() {
        // For AMO instructions: funct7[6:2] = amo_op (5 bits), funct7[1:0] = aq/rl (2 bits) = 0
        // funct5() returns (inst >> 27) & 0x1F = funct7[6:2] = amo_op
        
        // LR.W: amo_op = 0b00010 = 0x02
        assert_eq!(Instruction(0x1000_202F).mnemonic(), "lr.w");
        // SC.W: amo_op = 0b00011 = 0x03
        assert_eq!(Instruction(0x1800_202F).mnemonic(), "sc.w");
        // AMOADD.W: amo_op = 0b00000 = 0x00
        assert_eq!(Instruction(0x0000_202F).mnemonic(), "amoadd.w");
        // AMOSWAP.W: amo_op = 0b00001 = 0x01
        assert_eq!(Instruction(0x0800_202F).mnemonic(), "amoswap.w");
        // AMOXOR.W: amo_op = 0b00100 = 0x04
        assert_eq!(Instruction(0x2000_202F).mnemonic(), "amoxor.w");
        // AMOAND.W: amo_op = 0b01100 = 0x0C
        assert_eq!(Instruction(0x6000_202F).mnemonic(), "amoand.w");
        // AMOOR.W: amo_op = 0b01000 = 0x08
        assert_eq!(Instruction(0x4000_202F).mnemonic(), "amoor.w");
        // AMOMIN.W: amo_op = 0b10000 = 0x10
        assert_eq!(Instruction(0x8000_202F).mnemonic(), "amomin.w");
        // AMOMAX.W: amo_op = 0b10100 = 0x14
        assert_eq!(Instruction(0xA000_202F).mnemonic(), "amomax.w");
        // AMOMINU.W: amo_op = 0b11000 = 0x18
        assert_eq!(Instruction(0xC000_202F).mnemonic(), "amominu.w");
        // AMOMAXU.W: amo_op = 0b11100 = 0x1C
        assert_eq!(Instruction(0xE000_202F).mnemonic(), "amomaxu.w");
    }
}
