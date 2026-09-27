// src/cpu/c_decode.rs

/// Map 3-bit compressed register index (0..7) to standard x8..x15 (s0, s1, a0..a5)
#[inline(always)]
fn map_c_reg(r: u32) -> u32 {
    8 + (r & 0x7)
}

/// Sign-extend an immediate value of `bits` width
#[inline(always)]
fn sign_extend(val: u32, bits: u32) -> u32 {
    let shift = 32 - bits;
    ((val << shift) as i32 >> shift) as u32
}

/// Decompresses a 16-bit RV32C instruction into a standard 32-bit RV32I instruction
pub fn decompress(c_raw: u16) -> Option<u32> {
    let c = c_raw as u32; // Promoted to u32 so all intermediate math is u32
    let quad = c & 0x3;
    let funct3 = (c >> 13) & 0x7;

    match quad {
        // Quadrant 0 (inst[1:0] == 0b00)
        0b00 => match funct3 {
            // C.ADDI4SPN -> addi rd', x2, nzuimm
            0b000 => {
                let nzuimm = (((c >> 6) & 0x1) << 2)
                    | (((c >> 5) & 0x1) << 3)
                    | (((c >> 11) & 0x3) << 4)
                    | (((c >> 7) & 0xF) << 6);
                if nzuimm == 0 { return None; } // Reserved
                let rd_prime = map_c_reg((c >> 2) & 0x7);
                Some((nzuimm << 20) | (2 << 15) | (0b000 << 12) | (rd_prime << 7) | 0x13)
            }
            // C.LW -> lw rd', offset(rs1')
            0b010 => {
                let offset = (((c >> 6) & 0x1) << 2)
                    | (((c >> 10) & 0x7) << 3)
                    | (((c >> 5) & 0x1) << 6);
                let rs1_prime = map_c_reg((c >> 7) & 0x7);
                let rd_prime = map_c_reg((c >> 2) & 0x7);
                Some((offset << 20) | (rs1_prime << 15) | (0b010 << 12) | (rd_prime << 7) | 0x03)
            }
            // C.SW -> sw rs2', offset(rs1')
            0b110 => {
                let offset = (((c >> 6) & 0x1) << 2)
                    | (((c >> 10) & 0x7) << 3)
                    | (((c >> 5) & 0x1) << 6);
                let rs1_prime = map_c_reg((c >> 7) & 0x7);
                let rs2_prime = map_c_reg((c >> 2) & 0x7);
                let imm_11_5 = (offset >> 5) & 0x7F;
                let imm_4_0 = offset & 0x1F;
                Some((imm_11_5 << 25) | (rs2_prime << 20) | (rs1_prime << 15) | (0b010 << 12) | (imm_4_0 << 7) | 0x23)
            }
            _ => None,
        },

        // Quadrant 1 (inst[1:0] == 0b01)
        0b01 => match funct3 {
            // C.NOP / C.ADDI -> addi rd, rd, nzimm
            0b000 => {
                let rd = (c >> 7) & 0x1F;
                let imm = sign_extend(((c >> 2) & 0x1F) | (((c >> 12) & 0x1) << 5), 6);
                Some(((imm & 0xFFF) << 20) | (rd << 15) | (0b000 << 12) | (rd << 7) | 0x13)
            }
            // C.JAL (RV32C only) -> jal x1, offset
            0b001 => {
                let offset = (((c >> 3) & 0x7) << 1)
                    | (((c >> 11) & 0x1) << 4)
                    | (((c >> 2) & 0x1) << 5)
                    | (((c >> 7) & 0x1) << 6)
                    | (((c >> 6) & 0x1) << 7)
                    | (((c >> 9) & 0x3) << 8)
                    | (((c >> 8) & 0x1) << 10)
                    | (((c >> 12) & 0x1) << 11);
                let imm = sign_extend(offset, 12);
                let j_imm = (((imm >> 20) & 0x1) << 31)
                    | (((imm >> 1) & 0x3FF) << 21)
                    | (((imm >> 11) & 0x1) << 20)
                    | (((imm >> 12) & 0xFF) << 12);
                Some(j_imm | (1 << 7) | 0x6F)
            }
            // C.LI -> addi rd, x0, imm
            0b010 => {
                let rd = (c >> 7) & 0x1F;
                if rd == 0 { return None; }
                let imm = sign_extend(((c >> 2) & 0x1F) | (((c >> 12) & 0x1) << 5), 6);
                Some(((imm & 0xFFF) << 20) | (0 << 15) | (0b000 << 12) | (rd << 7) | 0x13)
            }
            // C.ADDI16SP / C.LUI
            0b011 => {
                let rd = (c >> 7) & 0x1F;
                if rd == 2 {
                    // C.ADDI16SP -> addi x2, x2, nzimm
                    let offset = (((c >> 6) & 0x1) << 4)
                        | (((c >> 2) & 0x1) << 5)
                        | (((c >> 5) & 0x1) << 6)
                        | (((c >> 3) & 0x3) << 7)
                        | (((c >> 12) & 0x1) << 9);
                    if offset == 0 { return None; }
                    let imm = sign_extend(offset, 10);
                    Some(((imm & 0xFFF) << 20) | (2 << 15) | (0b000 << 12) | (2 << 7) | 0x13)
                } else if rd != 0 {
                    // C.LUI -> lui rd, nzimm
                    let nzimm = ((c >> 2) & 0x1F) | (((c >> 12) & 0x1) << 5);
                    if nzimm == 0 { return None; }
                    let imm = sign_extend(nzimm, 6) << 12;
                    Some((imm & 0xFFFF_F000) | (rd << 7) | 0x37)
                } else {
                    None
                }
            }
            // C.MISC-ALU (SRLI, SRAI, ANDI, SUB, XOR, OR, AND)
            0b100 => {
                let funct2 = (c >> 10) & 0x3;
                let rd_prime = map_c_reg((c >> 7) & 0x7);
                match funct2 {
                    0b00 => {
                        // C.SRLI -> srli rd', rd', shamt
                        let shamt = ((c >> 2) & 0x1F) | (((c >> 12) & 0x1) << 5);
                        Some((shamt << 20) | (rd_prime << 15) | (0b101 << 12) | (rd_prime << 7) | 0x13)
                    }
                    0b01 => {
                        // C.SRAI -> srai rd', rd', shamt
                        let shamt = ((c >> 2) & 0x1F) | (((c >> 12) & 0x1) << 5);
                        Some((0x20 << 25) | (shamt << 20) | (rd_prime << 15) | (0b101 << 12) | (rd_prime << 7) | 0x13)
                    }
                    0b10 => {
                        // C.ANDI -> andi rd', rd', imm
                        let imm = sign_extend(((c >> 2) & 0x1F) | (((c >> 12) & 0x1) << 5), 6);
                        Some(((imm & 0xFFF) << 20) | (rd_prime << 15) | (0b111 << 12) | (rd_prime << 7) | 0x13)
                    }
                    0b11 => {
                        let rs2_prime = map_c_reg((c >> 2) & 0x7);
                        let sub_funct = ((c >> 5) & 0x3) | (((c >> 12) & 0x1) << 2);
                        match sub_funct {
                            0b000 => Some((0x20 << 25) | (rs2_prime << 20) | (rd_prime << 15) | (0b000 << 12) | (rd_prime << 7) | 0x33), // sub
                            0b001 => Some((0x00 << 25) | (rs2_prime << 20) | (rd_prime << 15) | (0b100 << 12) | (rd_prime << 7) | 0x33), // xor
                            0b010 => Some((0x00 << 25) | (rs2_prime << 20) | (rd_prime << 15) | (0b110 << 12) | (rd_prime << 7) | 0x33), // or
                            0b011 => Some((0x00 << 25) | (rs2_prime << 20) | (rd_prime << 15) | (0b111 << 12) | (rd_prime << 7) | 0x33), // and
                            _ => None,
                        }
                    }
                    _ => unreachable!(),
                }
            }
            // C.J -> jal x0, offset
            0b101 => {
                let offset = (((c >> 3) & 0x7) << 1)
                    | (((c >> 11) & 0x1) << 4)
                    | (((c >> 2) & 0x1) << 5)
                    | (((c >> 7) & 0x1) << 6)
                    | (((c >> 6) & 0x1) << 7)
                    | (((c >> 9) & 0x3) << 8)
                    | (((c >> 8) & 0x1) << 10)
                    | (((c >> 12) & 0x1) << 11);
                let imm = sign_extend(offset, 12);
                let j_imm = (((imm >> 20) & 0x1) << 31)
                    | (((imm >> 1) & 0x3FF) << 21)
                    | (((imm >> 11) & 0x1) << 20)
                    | (((imm >> 12) & 0xFF) << 12);
                Some(j_imm | (0 << 7) | 0x6F)
            }
            // C.BEQZ -> beq rs1', x0, offset
            0b110 => {
                let offset = (((c >> 3) & 0x3) << 1)
                    | (((c >> 10) & 0x3) << 3)
                    | (((c >> 2) & 0x1) << 5)
                    | (((c >> 5) & 0x3) << 6)
                    | (((c >> 12) & 0x1) << 8);
                let imm = sign_extend(offset, 9);
                let rs1_prime = map_c_reg((c >> 7) & 0x7);
                let b_imm_12 = (imm >> 12) & 0x1;
                let b_imm_10_5 = (imm >> 5) & 0x3F;
                let b_imm_4_1 = (imm >> 1) & 0xF;
                let b_imm_11 = (imm >> 11) & 0x1;
                Some((b_imm_12 << 31) | (b_imm_10_5 << 25) | (0 << 20) | (rs1_prime << 15) | (0b000 << 12) | (b_imm_4_1 << 8) | (b_imm_11 << 7) | 0x63)
            }
            // C.BNEZ -> bne rs1', x0, offset
            0b111 => {
                let offset = (((c >> 3) & 0x3) << 1)
                    | (((c >> 10) & 0x3) << 3)
                    | (((c >> 2) & 0x1) << 5)
                    | (((c >> 5) & 0x3) << 6)
                    | (((c >> 12) & 0x1) << 8);
                let imm = sign_extend(offset, 9);
                let rs1_prime = map_c_reg((c >> 7) & 0x7);
                let b_imm_12 = (imm >> 12) & 0x1;
                let b_imm_10_5 = (imm >> 5) & 0x3F;
                let b_imm_4_1 = (imm >> 1) & 0xF;
                let b_imm_11 = (imm >> 11) & 0x1;
                Some((b_imm_12 << 31) | (b_imm_10_5 << 25) | (0 << 20) | (rs1_prime << 15) | (0b001 << 12) | (b_imm_4_1 << 8) | (b_imm_11 << 7) | 0x63)
            }
            _ => None,
        },

        // Quadrant 2 (inst[1:0] == 0b10)
        0b10 => match funct3 {
            // C.SLLI -> slli rd, rd, shamt
            0b000 => {
                let rd = (c >> 7) & 0x1F;
                if rd == 0 { return None; }
                let shamt = ((c >> 2) & 0x1F) | (((c >> 12) & 0x1) << 5);
                Some((shamt << 20) | (rd << 15) | (0b001 << 12) | (rd << 7) | 0x13)
            }
            // C.LWSP -> lw rd, offset(x2)
            0b010 => {
                let rd = (c >> 7) & 0x1F;
                if rd == 0 { return None; }
                let offset = (((c >> 4) & 0x7) << 2)
                    | (((c >> 12) & 0x1) << 5)
                    | (((c >> 2) & 0x3) << 6);
                Some((offset << 20) | (2 << 15) | (0b010 << 12) | (rd << 7) | 0x03)
            }
            // C.JR / C.MV / C.EBREAK / C.JALR / C.ADD
            0b100 => {
                let bit12 = (c >> 12) & 0x1;
                let rs1 = (c >> 7) & 0x1F;
                let rs2 = (c >> 2) & 0x1F;

                if bit12 == 0 {
                    if rs2 == 0 {
                        // C.JR -> jalr x0, 0(rs1)
                        if rs1 == 0 { return None; }
                        Some((0 << 20) | (rs1 << 15) | (0b000 << 12) | (0 << 7) | 0x67)
                    } else {
                        // C.MV -> add rd, x0, rs2
                        Some((0 << 25) | (rs2 << 20) | (0 << 15) | (0b000 << 12) | (rs1 << 7) | 0x33)
                    }
                } else {
                    if rs1 == 0 && rs2 == 0 {
                        // C.EBREAK -> ebreak
                        Some(0x00100073)
                    } else if rs2 == 0 {
                        // C.JALR -> jalr x1, 0(rs1)
                        Some((0 << 20) | (rs1 << 15) | (0b000 << 12) | (1 << 7) | 0x67)
                    } else {
                        // C.ADD -> add rd, rd, rs2
                        Some((0 << 25) | (rs2 << 20) | (rs1 << 15) | (0b000 << 12) | (rs1 << 7) | 0x33)
                    }
                }
            }
            // C.SWSP -> sw rs2, offset(x2)
            0b110 => {
                let rs2 = (c >> 2) & 0x1F;
                let offset = (((c >> 9) & 0xF) << 2) | (((c >> 7) & 0x3) << 6);
                let imm_11_5 = (offset >> 5) & 0x7F;
                let imm_4_0 = offset & 0x1F;
                Some((imm_11_5 << 25) | (rs2 << 20) | (2 << 15) | (0b010 << 12) | (imm_4_0 << 7) | 0x23)
            }
            _ => None,
        },

        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::decompress;

    #[test]
    fn test_c_nop() {
        // C.NOP = C.ADDI with rd=x0, imm=0 -> 0x0001
        assert_eq!(decompress(0x0001), Some(0x0000_0013)); // addi x0, x0, 0
    }

    #[test]
    fn test_c_ebreak() {
        // C.EBREAK
        assert_eq!(decompress(0x9002), Some(0x0010_0073));
    }

    #[test]
    fn test_c_bez() {
        // C.BEQZ x8, offset=4
        let result = decompress(0xC401); // C.BEQZ x8, 4
        assert!(result.is_some());
        let inst = result.unwrap();
        assert_eq!(inst & 0x7F, 0x63); // BRANCH
        assert_eq!((inst >> 12) & 0x7, 0x0); // funct3=0 (BEQ)
        assert_eq!((inst >> 15) & 0x1F, 8); // rs1=x8
        assert_eq!((inst >> 20) & 0x1F, 0); // rs2=x0
    }

    #[test]
    fn test_c_bnez() {
        // C.BNEZ x8, offset=4
        let result = decompress(0xE401); // C.BNEZ x8, 4
        assert!(result.is_some());
        let inst = result.unwrap();
        assert_eq!((inst >> 12) & 0x7, 0x1); // funct3=1 (BNE)
    }

    #[test]
    fn test_c_j() {
        // C.J offset=4
        let result = decompress(0xA001); // C.J 4
        assert!(result.is_some());
        let inst = result.unwrap();
        assert_eq!(inst & 0x7F, 0x6F); // JAL
        assert_eq!((inst >> 7) & 0x1F, 0); // rd=x0
    }

    #[test]
    fn test_c_lwsp() {
        // C.LWSP x5, 8(x2)
        let result = decompress(0x4282); // C.LWSP x5, 8
        assert!(result.is_some());
        let inst = result.unwrap();
        assert_eq!(inst & 0x7F, 0x03); // LOAD
        assert_eq!((inst >> 12) & 0x7, 0x2); // funct3=2 (LW)
        assert_eq!((inst >> 7) & 0x1F, 5); // rd=x5
        assert_eq!((inst >> 15) & 0x1F, 2); // rs1=x2
    }

    #[test]
    fn test_c_addi4spn() {
        // C.ADDI4SPN x8, x2, 8 -> x8 = x2 + 8
        // funct3=000, rd'=0 (x8), nzuimm=8
        let result = decompress(0x0020); // C.ADDI4SPN x8, 8
        assert!(result.is_some());
        let inst = result.unwrap();
        assert_eq!(inst & 0x7F, 0x13); // OP-IMM
        assert_eq!((inst >> 7) & 0x1F, 8); // rd=x8
        assert_eq!((inst >> 15) & 0x1F, 2); // rs1=x2
    }

    #[test]
    fn test_invalid_instructions() {
        // Reserved: C.ADDI4SPN with nzuimm=0
        assert_eq!(decompress(0x0000), None);
        
        // C.LI with rd=0
        assert_eq!(decompress(0x0002), None);
        
        // C.LUI with rd=0
        assert_eq!(decompress(0x0003), None);
        
        // C.LWSP with rd=0
        assert_eq!(decompress(0x0012), None);
        
        // C.JR with rs1=0
        assert_eq!(decompress(0x8000), None);
    }
}