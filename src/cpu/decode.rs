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
