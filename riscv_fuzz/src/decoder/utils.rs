//! Utility functions for instruction decoding

/// Sign-extend a value of specified bit width to i32
pub fn sign_extend(value: u32, width: u8) -> i32 {
    let sign_bit = 1u32 << (width - 1);
    let max_value = 1u32 << width;
    
    if (sign_bit & value) != 0 {
        value as i32 - max_value as i32
    } else {
        value as i32
    }
}

/// Extract immediate value for I-type instructions
pub fn extract_i_immediate(inst: u32) -> i32 {
    sign_extend((inst >> 20) & 0xFFF, 12)
}

/// Extract immediate value for S-type instructions  
pub fn extract_s_immediate(inst: u32) -> i32 {
    let imm_11_5 = (inst >> 25) & 0x7F;
    let imm_4_0 = (inst >> 7) & 0x1F;
    sign_extend((imm_11_5 << 5) | imm_4_0, 12)
}

/// Extract immediate value for B-type instructions
pub fn extract_b_immediate(inst: u32) -> i32 {
    let imm_12 = (inst >> 31) & 0x1;
    let imm_11 = (inst >> 7) & 0x1;
    let imm_10_5 = (inst >> 25) & 0x3F;
    let imm_4_1 = (inst >> 8) & 0xF;
    
    let imm = (imm_12 << 12) | (imm_11 << 11) | (imm_10_5 << 5) | (imm_4_1 << 1);
    sign_extend(imm, 13)
}

/// Extract immediate value for U-type instructions
pub fn extract_u_immediate(inst: u32) -> i32 {
    (inst & 0xFFFFF000) as i32
}

/// Extract immediate value for J-type instructions
pub fn extract_j_immediate(inst: u32) -> i32 {
    let imm_20 = (inst >> 31) & 0x1;
    let imm_19_12 = (inst >> 12) & 0xFF;
    let imm_11 = (inst >> 20) & 0x1;
    let imm_10_1 = (inst >> 21) & 0x3FF;
    
    let imm = (imm_20 << 20) | (imm_19_12 << 12) | (imm_11 << 11) | (imm_10_1 << 1);
    sign_extend(imm, 21)
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_sign_extend() {
        // Positive number
        assert_eq!(sign_extend(0x123, 12), 0x123);
        
        // Negative number (sign bit set)
        assert_eq!(sign_extend(0x800, 12), -2048);
        assert_eq!(sign_extend(0xFFF, 12), -1);
    }
    
    #[test]
    fn test_extract_i_immediate() {
        // addi x1, x0, 42
        let inst = 0x02A00093;  // imm=42, rs1=0, funct3=0, rd=1, opcode=0x13
        assert_eq!(extract_i_immediate(inst), 42);
        
        // addi x1, x0, -1
        let inst = 0xFFF00093;  // imm=-1, rs1=0, funct3=0, rd=1, opcode=0x13
        assert_eq!(extract_i_immediate(inst), -1);
    }
}