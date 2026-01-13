//! Compressed (16-bit) RISC-V instruction decoder
//!
//! Implements decoding for the RVC (compressed) extension which provides
//! 16-bit encodings for common RISC-V instructions to improve code density.
//!
//! Compressed instructions are organized by quadrants based on bits [1:0]:
//! - Quadrant 0 (00): Stack-pointer based loads/stores, wide immediates
//! - Quadrant 1 (01): Control transfers, integer constants and computations
//! - Quadrant 2 (10): Stack-pointer based operations, register moves
//! - Quadrant 3 (11): Reserved for 32-bit instructions

pub mod error;
pub mod instruction;

pub use error::DecodeError;
pub use instruction::Instruction;

use crate::target::Target;

/// Bit masks for compressed instruction field extraction
const MASK1: u16 = 0b1; // 1-bit mask
const MASK2: u16 = 0b11; // 2-bit mask
const MASK3: u16 = 0b111; // 3-bit mask
const MASK4: u16 = 0b1111; // 4-bit mask
const MASK5: u16 = 0b11111; // 5-bit mask

#[inline(always)]
/// Compressed instructions can be identified by checking that the
/// las two bits in the instruction are not `0b11`
pub fn is_compressed(bits: u16) -> bool {
    (bits & MASK2) != 0x3
}

/// Decode a 16-bit compressed RISC-V instruction
pub fn decode_compressed_instruction(
    bits: u16,
    target: &Target,
) -> Result<Instruction, DecodeError> {
    // Handle special case: all zeros = compressed illegal
    if bits == 0 {
        return Ok(Instruction::C_ILLEGAL);
    }

    // Parse instruction fields
    let encoded = EncodedInstruction::new(bits);

    // Decode based on quadrant (bits [1:0])
    match encoded.quadrant {
        0 => decode_quadrant_0(&encoded, target),
        1 => decode_quadrant_1(&encoded, target),
        2 => decode_quadrant_2(&encoded, target),
        3 => Err(DecodeError::NotCompressed), // 32-bit instruction
        _ => unreachable!("Quadrant can only be 0-3"),
    }
}

/// Encoded compressed instruction with extracted fields
struct EncodedInstruction {
    bits: u16,
    quadrant: u8,
    funct3: u8,
    rd: u8,
    rs2: u8,
    rd_prime: u8,  // Compressed register (3-bit)
    rs1_prime: u8, // Compressed register (3-bit)
    rs2_prime: u8, // Compressed register (3-bit)
    /// CI shift amount (6-bit): shamt[4:0] = bits[6:2], shamt[5] = bit[12]
    shamt6: u8,
    // TODO: not that readable?
    /// CL-format LW uimm (offset)
    uimm_cl_lw: u8,
    /// CL-format LD uimm (offset)
    uimm_cl_ld: u8,
    /// CS-format SW uimm (offset)
    uimm_cs_sw: u8,
    /// CS-format SD uimm (offset)
    uimm_cs_sd: u8,
    /// CI-format sign-extended immediate used by various CI ops (bits[12|6:2])
    ci_imm: i32,
    /// CI-format LWSP uimm (offset)
    uimm_ci_lwsp: u8,
    /// CI-format LDSP uimm (offset)
    uimm_ci_ldsp: u8,
    /// CSS-format SWSP uimm (offset)
    uimm_css_swsp: u8,
    /// CSS-format SDSP uimm (offset)
    uimm_css_sdsp: u8,
    /// CJ-format signed jump offset
    cj_offset: i16,
    /// CB-format signed branch offset
    cb_offset: i16,
}

impl EncodedInstruction {
    /// Parse all possible fields from a 16-bit compressed instruction
    fn new(bits: u16) -> Self {
        /*
        Below are the compressed instruction formats that all 16-bit
        instructions fit into, organized by quadrant (bits [1:0]).

        Note: Compressed instructions use different register encoding:
        - rd/rs1/rs2 (5-bit): Full register x0-x31
        - rd'/rs1'/rs2' (3-bit): Compressed registers x8-x15

        EncodedInstruction extracts all possible fields, then we choose
        the appropriate ones based on the quadrant and funct3 during decoding.

        CR-type | funct4 |   rd/rs1   |   rs2    | op |
                | 15-12  |    11-7    |   6-2    | 1-0|
        ------------------------------------------------

        CI-type | funct3 | imm |   rd/rs1   | imm | op |
                | 15-13  | 12  |    11-7    | 6-2 | 1-0|
        ------------------------------------------------

        CSS-type| funct3 |     imm     |   rs2    | op |
                | 15-13  |    12-7     |   6-2    | 1-0|
        ------------------------------------------------

        CIW-type| funct3 |     imm      | rd' | op |
                | 15-13  |     12-5     | 4-2 | 1-0|
        ------------------------------------------

        CL-type | funct3 | imm | rs1' | imm | rd' | op |
                | 15-13  |12-10| 9-7  | 6-5 | 4-2 | 1-0|
        ------------------------------------------------

        CS-type | funct3 | imm | rs1' | imm | rs2'| op |
                | 15-13  |12-10| 9-7  | 6-5 | 4-2 | 1-0|
        ------------------------------------------------

        CA-type | funct6 | rd'/rs1' | funct2 | rs2'| op |
                | 15-10  |   9-7    |  6-5   | 4-2 | 1-0|
        ------------------------------------------------

        CB-type | funct3 | off | rs1' |    offset    | op |
                | 15-13  | 12  | 9-7  |   6-2        | 1-0|
        ------------------------------------------------

        CJ-type | funct3 |        jump target        | op |
                | 15-13  |         12-2              | 1-0|
        ------------------------------------------------
        */

        // Extract special fields
        let shamt6 = extract_ci_shift_immediate(bits);
        let uimm_cl_lw = extract_cl_lw_offset(bits);
        let uimm_cl_ld = extract_cl_ld_offset(bits);
        let uimm_cs_sw = extract_cs_sw_offset(bits);
        let uimm_cs_sd = extract_cs_sd_offset(bits);
        let ci_imm = extract_ci_immediate(bits);
        let uimm_ci_lwsp = extract_ci_lwsp_offset(bits);
        let uimm_ci_ldsp = extract_ci_ldsp_offset(bits);
        let uimm_css_swsp = extract_css_swsp_offset(bits);
        let uimm_css_sdsp = extract_css_sdsp_offset(bits);
        let cj_offset = extract_cj_offset(bits);
        let cb_offset = extract_cb_offset(bits);

        Self {
            bits,
            quadrant: (bits & MASK2) as u8,
            funct3: ((bits >> 13) & MASK3) as u8,
            rd: ((bits >> 7) & MASK5) as u8,
            rs2: ((bits >> 2) & MASK5) as u8,
            rd_prime: ((bits >> 2) & MASK3) as u8,
            rs1_prime: ((bits >> 7) & MASK3) as u8,
            rs2_prime: ((bits >> 2) & MASK3) as u8,
            shamt6,
            uimm_cl_lw,
            uimm_cl_ld,
            uimm_cs_sw,
            uimm_cs_sd,
            ci_imm,
            uimm_ci_lwsp,
            uimm_ci_ldsp,
            uimm_css_swsp,
            uimm_css_sdsp,
            cj_offset,
            cb_offset,
        }
    }
}

/// Convert compressed register index (3-bit) to full register index (x8-x15)
fn expand_compressed_reg(reg: u8) -> u8 {
    // TODO: add an assert to check that top 5 bits are 0. EncodedInstruction guarantees this.
    8 + (reg & MASK3 as u8)
}

/// Decode Quadrant 0 instructions (bits [1:0] = 00)
fn decode_quadrant_0(
    encoded: &EncodedInstruction,
    target: &Target,
) -> Result<Instruction, DecodeError> {
    let rd = expand_compressed_reg(encoded.rd_prime);
    let rs1 = expand_compressed_reg(encoded.rs1_prime);
    let rs2 = expand_compressed_reg(encoded.rs2_prime);
    match encoded.funct3 {
        0b000 => {
            // C.ADDI4SPN - Add immediate to stack pointer, non-zero
            let nzuimm = extract_ciw_immediate(encoded.bits);
            if nzuimm == 0 {
                return Err(DecodeError::Reserved);
            }
            Ok(Instruction::C_ADDI4SPN { rd, imm: nzuimm })
        }
        0b010 => {
            // C.LW - Load word
            let offset = encoded.uimm_cl_lw;
            Ok(Instruction::C_LW { rd, rs1, offset })
        }
        0b011 => {
            // C.LD - Load doubleword (RV64/128 only)
            if !target.supports_extension(crate::target::Extension::RV64I) {
                return Err(DecodeError::UnsupportedOnTarget);
            }
            let offset = encoded.uimm_cl_ld;
            Ok(Instruction::C_LD { rd, rs1, offset })
        }
        0b110 => {
            // C.SW - Store word
            let offset = encoded.uimm_cs_sw;
            Ok(Instruction::C_SW { rs1, rs2, offset })
        }
        0b111 => {
            // C.SD - Store doubleword (RV64/128 only)
            if !target.supports_extension(crate::target::Extension::RV64I) {
                return Err(DecodeError::UnsupportedOnTarget);
            }
            let offset = encoded.uimm_cs_sd;
            Ok(Instruction::C_SD { rs1, rs2, offset })
        }
        _ => Err(DecodeError::InvalidInstruction),
    }
}

/// Decode Quadrant 1 instructions (bits [1:0] = 01)
fn decode_quadrant_1(
    encoded: &EncodedInstruction,
    target: &Target,
) -> Result<Instruction, DecodeError> {
    let rs1p = expand_compressed_reg(encoded.rs1_prime);
    match encoded.funct3 {
        0b000 => {
            // C.NOP or C.ADDI
            if encoded.rd == 0 {
                Ok(Instruction::C_NOP)
            } else {
                let imm = encoded.ci_imm;
                Ok(Instruction::C_ADDI { rd: encoded.rd, imm: imm as i8 })
            }
        }
        0b001 => {
            // C.JAL (RV32 only) or C.ADDIW (RV64/128)
            if target.supports_extension(crate::target::Extension::RV64I) {
                // C.ADDIW
                if encoded.rd == 0 {
                    return Err(DecodeError::Reserved);
                }
                let imm = encoded.ci_imm;
                Ok(Instruction::C_ADDIW { rd: encoded.rd, imm: imm as i8 })
            } else {
                // C.JAL (RV32 only)
                Ok(Instruction::C_JAL { offset: encoded.cj_offset })
            }
        }
        0b010 => {
            // C.LI - Load immediate
            let imm = encoded.ci_imm;
            Ok(Instruction::C_LI { rd: encoded.rd, imm: imm as i8 })
        }
        0b011 => {
            // C.ADDI16SP or C.LUI
            if encoded.rd == 2 {
                // C.ADDI16SP
                let imm = extract_ci16sp_immediate(encoded.bits);
                if imm == 0 {
                    return Err(DecodeError::Reserved);
                }
                Ok(Instruction::C_ADDI16SP { imm })
            } else if encoded.rd != 0 {
                // C.LUI
                let imm = extract_ci_lui_immediate(encoded.bits);
                if imm == 0 {
                    return Err(DecodeError::Reserved);
                }
                Ok(Instruction::C_LUI { rd: encoded.rd, imm })
            } else {
                Err(DecodeError::Reserved)
            }
        }
        0b100 => decode_quadrant_1_misc_alu(encoded, target),
        0b101 => {
            // C.J - Jump
            Ok(Instruction::C_J { offset: encoded.cj_offset })
        }
        0b110 => {
            // C.BEQZ - Branch if equal zero
            Ok(Instruction::C_BEQZ { rs1: rs1p, offset: encoded.cb_offset as i8 })
        }
        0b111 => {
            // C.BNEZ - Branch if not equal zero
            Ok(Instruction::C_BNEZ { rs1: rs1p, offset: encoded.cb_offset as i8 })
        }
        _ => unreachable!(),
    }
}

/// Decode Quadrant 1 miscellaneous ALU instructions (funct3 = 100)
fn decode_quadrant_1_misc_alu(
    encoded: &EncodedInstruction,
    target: &Target,
) -> Result<Instruction, DecodeError> {
    let funct2 = (encoded.bits >> 10) & MASK2;
    let rd_p = expand_compressed_reg(encoded.rs1_prime);
    let rs2_p = expand_compressed_reg(encoded.rs2_prime);
    match funct2 {
        0b00 => {
            // C.SRLI - Shift right logical immediate
            let shamt = encoded.shamt6;
            // RV32: shamt[5] must be zero
            if !target.supports_extension(crate::target::Extension::RV64I)
                && (shamt & 0b100000) != 0
            {
                return Err(DecodeError::Reserved);
            }
            Ok(Instruction::C_SRLI { rd: rd_p, shamt })
        }
        0b01 => {
            // C.SRAI - Shift right arithmetic immediate
            let shamt = encoded.shamt6;
            // RV32: shamt[5] must be zero
            if !target.supports_extension(crate::target::Extension::RV64I)
                && (shamt & 0b100000) != 0
            {
                return Err(DecodeError::Reserved);
            }
            Ok(Instruction::C_SRAI { rd: rd_p, shamt })
        }
        0b10 => {
            // C.ANDI - AND immediate
            let imm = encoded.ci_imm;
            Ok(Instruction::C_ANDI { rd: rd_p, imm: imm as i8 })
        }
        0b11 => {
            // Register-register operations
            let funct2_low = (encoded.bits >> 5) & MASK2;
            let funct1 = (encoded.bits >> 12) & MASK1;

            match (funct1, funct2_low) {
                (0, 0b00) => Ok(Instruction::C_SUB { rd: rd_p, rs2: rs2_p }),
                (0, 0b01) => Ok(Instruction::C_XOR { rd: rd_p, rs2: rs2_p }),
                (0, 0b10) => Ok(Instruction::C_OR { rd: rd_p, rs2: rs2_p }),
                (0, 0b11) => Ok(Instruction::C_AND { rd: rd_p, rs2: rs2_p }),
                (1, 0b00) => {
                    // C.SUBW is RV64/128 only
                    if !target.supports_extension(crate::target::Extension::RV64I) {
                        return Err(DecodeError::UnsupportedOnTarget);
                    }
                    Ok(Instruction::C_SUBW { rd: rd_p, rs2: rs2_p })
                }
                (1, 0b01) => {
                    // C.ADDW is RV64/128 only
                    if !target.supports_extension(crate::target::Extension::RV64I) {
                        return Err(DecodeError::UnsupportedOnTarget);
                    }
                    Ok(Instruction::C_ADDW { rd: rd_p, rs2: rs2_p })
                }
                _ => Err(DecodeError::Reserved),
            }
        }
        _ => unreachable!(),
    }
}

/// Decode Quadrant 2 instructions (bits [1:0] = 10)
fn decode_quadrant_2(
    encoded: &EncodedInstruction,
    target: &Target,
) -> Result<Instruction, DecodeError> {
    let rd = encoded.rd;
    let rs2 = encoded.rs2;
    match encoded.funct3 {
        0b000 => {
            // C.SLLI - Shift left logical immediate
            let shamt = encoded.shamt6;
            // RV32: shamt[5] must be zero
            if !target.supports_extension(crate::target::Extension::RV64I)
                && (shamt & 0b100000) != 0
            {
                return Err(DecodeError::Reserved);
            }
            Ok(Instruction::C_SLLI { rd, shamt })
        }
        0b010 => {
            // C.LWSP - Load word from stack pointer
            if rd == 0 {
                return Err(DecodeError::Reserved);
            }
            let offset = encoded.uimm_ci_lwsp;
            Ok(Instruction::C_LWSP { rd, offset })
        }
        0b011 => {
            // C.LDSP - Load doubleword from stack pointer (RV64/128)
            if !target.supports_extension(crate::target::Extension::RV64I) {
                return Err(DecodeError::UnsupportedOnTarget);
            }
            if rd == 0 {
                return Err(DecodeError::Reserved);
            }
            let offset = encoded.uimm_ci_ldsp;
            Ok(Instruction::C_LDSP { rd, offset })
        }
        0b100 => decode_quadrant_2_misc(encoded),
        0b110 => {
            // C.SWSP - Store word to stack pointer
            let offset = encoded.uimm_css_swsp;
            Ok(Instruction::C_SWSP { rs2, offset })
        }
        0b111 => {
            // C.SDSP - Store doubleword to stack pointer (RV64/128)
            if !target.supports_extension(crate::target::Extension::RV64I) {
                return Err(DecodeError::UnsupportedOnTarget);
            }
            let offset = encoded.uimm_css_sdsp;
            Ok(Instruction::C_SDSP { rs2, offset })
        }
        _ => Err(DecodeError::InvalidInstruction),
    }
}

/// Decode Quadrant 2 miscellaneous instructions (funct3 = 100)
fn decode_quadrant_2_misc(encoded: &EncodedInstruction) -> Result<Instruction, DecodeError> {
    let funct1 = (encoded.bits >> 12) & MASK1;
    let rd = encoded.rd;
    let rs2 = encoded.rs2;

    if funct1 == 0 {
        // C.JR or C.MV
        if rs2 == 0 {
            // C.JR
            if rd == 0 {
                return Err(DecodeError::Reserved);
            }
            Ok(Instruction::C_JR { rs1: rd })
        } else {
            // C.MV
            Ok(Instruction::C_MV { rd, rs2 })
        }
    } else {
        // C.EBREAK, C.JALR, or C.ADD
        if rd == 0 && rs2 == 0 {
            // C.EBREAK
            Ok(Instruction::C_EBREAK)
        } else if rs2 == 0 {
            // C.JALR
            Ok(Instruction::C_JALR { rs1: rd })
        } else {
            // C.ADD
            Ok(Instruction::C_ADD { rd, rs2 })
        }
    }
}

// Immediate extraction functions

/// Extract CIW-format immediate for C.ADDI4SPN
/// The immediate represents nzuimm[9:2], so bits [1:0] are always 0
fn extract_ciw_immediate(bits: u16) -> u16 {
    let mut imm = 0u16;
    imm |= ((bits >> 7) & MASK4) << 6; // bits[10:7] -> imm[9:6]
    imm |= ((bits >> 11) & MASK2) << 4; // bits[12:11] -> imm[5:4]
    imm |= ((bits >> 5) & MASK1) << 3; // bit[5] -> imm[3]
    imm |= ((bits >> 6) & MASK1) << 2; // bit[6] -> imm[2]
                                       // imm[1:0] are always 0 for this instruction
    imm
}

/// Extract CL-format offset for C.LW
fn extract_cl_lw_offset(bits: u16) -> u8 {
    let mut offset = 0u8;
    offset |= (((bits >> 10) & MASK3) << 3) as u8; // bits[12:10] -> offset[5:3]
    offset |= (((bits >> 6) & MASK1) << 2) as u8; // bit[6] -> offset[2]
    offset |= (((bits >> 5) & MASK1) << 6) as u8; // bit[5] -> offset[6]
    offset
}

/// Extract CL-format offset for C.LD
fn extract_cl_ld_offset(bits: u16) -> u8 {
    let mut offset = 0u8;
    offset |= (((bits >> 10) & MASK3) << 3) as u8; // bits[12:10] -> offset[5:3]
    offset |= (((bits >> 5) & MASK2) << 6) as u8; // bits[6:5] -> offset[7:6]
    offset
}

/// Extract CS-format offset for C.SW
fn extract_cs_sw_offset(bits: u16) -> u8 {
    let mut offset = 0u8;
    offset |= (((bits >> 10) & MASK3) << 3) as u8; // bits[12:10] -> offset[5:3]
    offset |= (((bits >> 6) & MASK1) << 2) as u8; // bit[6] -> offset[2]
    offset |= (((bits >> 5) & MASK1) << 6) as u8; // bit[5] -> offset[6]
    offset
}

/// Extract CS-format offset for C.SD
fn extract_cs_sd_offset(bits: u16) -> u8 {
    let mut offset = 0u8;
    offset |= (((bits >> 10) & MASK3) << 3) as u8; // bits[12:10] -> offset[5:3]
    offset |= (((bits >> 5) & MASK2) << 6) as u8; // bits[6:5] -> offset[7:6]
    offset
}

/// Extract CI-format immediate
fn extract_ci_immediate(bits: u16) -> i32 {
    let mut imm = 0i32;
    imm |= ((bits >> 2) & MASK5) as i32; // bits[6:2] -> imm[4:0]
    imm |= (((bits >> 12) & MASK1) as i32) << 5; // bit[12] -> imm[5]
    // Sign-extend 6-bit immediate
    (imm << (32 - 6)) >> (32 - 6)
}

/// Extract CI-format immediate for C.ADDI16SP
fn extract_ci16sp_immediate(bits: u16) -> i16 {
    let mut imm = 0i16;
    imm |= (((bits >> 6) & MASK1) as i16) << 4; // bit[6] -> imm[4]
    imm |= (((bits >> 2) & MASK1) as i16) << 5; // bit[2] -> imm[5]
    imm |= (((bits >> 5) & MASK1) as i16) << 6; // bit[5] -> imm[6]
    imm |= (((bits >> 3) & MASK2) as i16) << 7; // bits[4:3] -> imm[8:7]
    imm |= (((bits >> 12) & MASK1) as i16) << 9; // bit[12] -> imm[9]
    // Sign-extend 10-bit immediate (bit 9 is sign)
    (imm << (16 - 10)) >> (16 - 10)
}

/// Extract CI-format immediate for C.LUI
fn extract_ci_lui_immediate(bits: u16) -> i32 {
    let mut imm = 0i32;
    imm |= (((bits >> 2) & MASK5) as i32) << 12; // bits[6:2] -> imm[16:12]
    imm |= (((bits >> 12) & MASK1) as i32) << 17; // bit[12] -> imm[17]
    // Sign-extend 18-bit immediate (bit 17 is sign)
    (imm << (32 - 18)) >> (32 - 18)
}

/// Extract CJ-format offset
fn extract_cj_offset(bits: u16) -> i16 {
    let mut offset = 0i16;
    offset |= (((bits >> 3) & MASK3) as i16) << 1; // bits[5:3] -> offset[3:1]
    offset |= (((bits >> 11) & MASK1) as i16) << 4; // bit[11] -> offset[4]
    offset |= (((bits >> 2) & MASK1) as i16) << 5; // bit[2] -> offset[5]
    offset |= (((bits >> 7) & MASK1) as i16) << 6; // bit[7] -> offset[6]
    offset |= (((bits >> 6) & MASK1) as i16) << 7; // bit[6] -> offset[7]
    offset |= (((bits >> 9) & MASK2) as i16) << 8; // bits[10:9] -> offset[9:8]
    offset |= (((bits >> 8) & MASK1) as i16) << 10; // bit[8] -> offset[10]
    offset |= (((bits >> 12) & MASK1) as i16) << 11; // bit[12] -> offset[11]
    // Sign-extend 12-bit offset (bit 11 is sign)
    (offset << (16 - 12)) >> (16 - 12)
}

/// Extract CB-format offset
fn extract_cb_offset(bits: u16) -> i16 {
    let mut offset = 0i16;
    offset |= (((bits >> 3) & MASK2) as i16) << 1; // bits[4:3] -> offset[2:1]
    offset |= (((bits >> 10) & MASK2) as i16) << 3; // bits[11:10] -> offset[4:3]
    offset |= (((bits >> 2) & MASK1) as i16) << 5; // bit[2] -> offset[5]
    offset |= (((bits >> 5) & MASK2) as i16) << 6; // bits[6:5] -> offset[7:6]
    offset |= (((bits >> 12) & MASK1) as i16) << 8; // bit[12] -> offset[8]
    // Sign-extend 9-bit offset (bit 8 is sign)
    (offset << (16 - 9)) >> (16 - 9)
}

/// Extract shift immediate for compressed shift operations
fn extract_ci_shift_immediate(bits: u16) -> u8 {
    let mut shamt = 0u8;
    shamt |= ((bits >> 2) & MASK5) as u8; // bits[6:2] -> shamt[4:0]
    shamt |= (((bits >> 12) & MASK1) as u8) << 5; // bit[12] -> shamt[5]
    shamt
}

/// Extract CSS-format offset for C.LWSP
fn extract_ci_lwsp_offset(bits: u16) -> u8 {
    let mut offset = 0u8;
    offset |= (((bits >> 4) & MASK3) as u8) << 2; // bits[6:4] -> offset[4:2]
    offset |= (((bits >> 12) & MASK1) as u8) << 5; // bit[12] -> offset[5]
    offset |= (((bits >> 2) & MASK2) as u8) << 6; // bits[3:2] -> offset[7:6]
    offset
}

/// Extract CSS-format offset for C.LDSP
fn extract_ci_ldsp_offset(bits: u16) -> u8 {
    let mut offset = 0u8;
    offset |= (((bits >> 5) & MASK2) as u8) << 3; // bits[6:5] -> offset[4:3]
    offset |= (((bits >> 12) & MASK1) as u8) << 5; // bit[12] -> offset[5]
    offset |= (((bits >> 2) & MASK3) as u8) << 6; // bits[4:2] -> offset[8:6]
    offset
}

/// Extract CSS-format offset for C.SWSP
fn extract_css_swsp_offset(bits: u16) -> u8 {
    let mut offset = 0u8;
    offset |= (((bits >> 9) & MASK4) as u8) << 2; // bits[12:9] -> offset[5:2]
    offset |= (((bits >> 7) & MASK2) as u8) << 6; // bits[8:7] -> offset[7:6]
    offset
}

/// Extract CSS-format offset for C.SDSP
fn extract_css_sdsp_offset(bits: u16) -> u8 {
    let mut offset = 0u8;
    offset |= (((bits >> 10) & MASK3) as u8) << 3; // bits[12:10] -> offset[5:3]
    offset |= (((bits >> 7) & MASK3) as u8) << 6; // bits[9:7] -> offset[8:6]
    offset
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_expand_compressed_reg_exhaustive() {
        // Test all 8 possible 3-bit compressed register values
        assert_eq!(expand_compressed_reg(0), 8); // x8
        assert_eq!(expand_compressed_reg(1), 9); // x9
        assert_eq!(expand_compressed_reg(2), 10); // x10
        assert_eq!(expand_compressed_reg(3), 11); // x11
        assert_eq!(expand_compressed_reg(4), 12); // x12
        assert_eq!(expand_compressed_reg(5), 13); // x13
        assert_eq!(expand_compressed_reg(6), 14); // x14
        assert_eq!(expand_compressed_reg(7), 15); // x15

        // Test that values > 7 are masked to 3 bits
        assert_eq!(expand_compressed_reg(8), 8); // 8 & 0x7 = 0 -> x8
        assert_eq!(expand_compressed_reg(15), 15); // 15 & 0x7 = 7 -> x15
        assert_eq!(expand_compressed_reg(255), 15); // 255 & 0x7 = 7 -> x15
    }
}
