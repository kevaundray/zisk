//! Standard (32-bit uncompressed) RISC-V instruction decoder
//! TODO: riscv manual has `fm` for the fence check and add. can also reference: https://five-embeddev.com/riscv-user-isa-manual/Priv-v1.12/rv32.html#rv32
//!
//! TODO(note): The public API of this module is `decode_standard_instruction` and its types.
//! TODO: Add hint instructions -- see section 2.9 and 4.4
mod error;
mod instruction;
mod opcode;

pub use error::DecodeError;
pub use instruction::Instruction;

use crate::{
    standard_decoder::opcode::{InstructionFormat, Opcode},
    target::{Extension, Target},
};

/// Decode a 32-bit standard RISC-V instruction
pub fn decode_standard_instruction(bits: u32, target: &Target) -> Result<Instruction, DecodeError> {
    // Handle special case: all zeros = illegal
    if bits == 0 {
        return Ok(Instruction::illegal());
    }

    // Parse all instruction fields
    let encoded = EncodedInstruction::new(bits);

    // Decode based on opcode enum
    match encoded.opcode {
        Some(Opcode::Load) => decode_load_instruction(&encoded, target),
        Some(Opcode::MiscMem) => decode_fence_instruction(&encoded, target),
        Some(Opcode::OpImm) => decode_op_imm_instruction(&encoded, target),
        Some(Opcode::Auipc) => decode_auipc_instruction(&encoded),
        Some(Opcode::Store) => decode_store_instruction(&encoded, target),
        Some(Opcode::Branch) => decode_branch_instruction(&encoded),
        Some(Opcode::Jalr) => decode_jalr_instruction(&encoded),
        Some(Opcode::Jal) => decode_jal_instruction(&encoded),
        Some(Opcode::Lui) => decode_lui_instruction(&encoded),
        Some(Opcode::System) => decode_system_instruction(&encoded, target),
        Some(Opcode::OpImm32) => decode_op_imm_32_instruction(&encoded, target),
        Some(Opcode::Amo) => decode_amo_instruction(&encoded, target),
        Some(Opcode::Op) => decode_op_instruction(&encoded, target),
        Some(Opcode::Op32) => decode_op_32_instruction(&encoded, target),

        None => Err(DecodeError::UnsupportedInstruction),
    }
}

/// Bit masks for field extraction
const MASK1: u32 = 0b1; // 1-bit mask
const MASK3: u32 = 0b111; // 3-bit mask
const MASK4: u32 = 0b1111; // 4-bit mask
const MASK5: u32 = 0b1_1111; // 5-bit mask
const MASK6: u32 = 0b11_1111; // 6-bit mask
const MASK7: u32 = 0b111_1111; // 7-bit mask
const MASK8: u32 = 0b1111_1111; // 8-bit mask
const MASK10: u32 = 0b11_1111_1111; // 10-bit mask
const MASK12: u32 = 0b1111_1111_1111; // 12-bit mask

/// Parsed fields from a 32-bit RISC-V instruction
///
/// This can be seen as a union of all of the formats, and then the decoder
/// picks the relevant fields based on the opcode.
///
/// This greatly simplifies the decoding methods.
///
/// Note: This does mean that redundant work is being done, for example
/// `aq` is being extracted each time, when it is only relevant for atomic
/// instructions.
/// This redundant work however is acceptable because bitwise operations
/// are fast.
#[derive(Debug, Clone, PartialEq)]
struct EncodedInstruction {
    /// Original 32-bit instruction word
    pub raw: u32,

    /// Opcode field (bits [6:0]) as raw value
    pub opcode_raw: u8,

    /// Opcode as enum (if recognized)
    pub opcode: Option<Opcode>,

    /// Destination register (bits [11:7])
    pub rd: u8,

    /// Function code 3 (bits [14:12])
    pub funct3: u8,

    /// Source register 1 (bits [19:15])
    pub rs1: u8,

    /// Source register 2 (bits [24:20])
    pub rs2: u8,

    /// Function code 7 (bits [31:25])
    pub funct7: u8,

    /// I-type immediate (bits [31:20], sign-extended)
    pub i_immediate: i32,

    /// S-type immediate (split across bits [31:25] and [11:7], sign-extended)
    pub s_immediate: i32,

    /// B-type immediate (branch offset, sign-extended)
    pub b_immediate: i32,

    /// U-type immediate (bits [31:12], left-shifted by 12)
    pub u_immediate: i32,

    /// J-type immediate (jump offset, sign-extended)
    pub j_immediate: i32,

    /// CSR address (bits [31:20]) for system instructions
    pub csr: u16,

    /// Shift amount for RV32I (5-bit, bits [24:20])
    pub shamt32: u8,

    /// Shift amount for RV64I (6-bit, bits [25:20])
    pub shamt64: u8,

    /// Acquire bit (bit [26]) for atomic instructions
    pub aq: bool,

    /// Release bit (bit [25]) for atomic instructions  
    pub rl: bool,

    /// Function code 5 (bits [31:27]) for atomic instructions
    pub funct5: u8,

    /// Predecessor field (bits [27:24]) for fence instructions
    pub pred: u8,

    /// Successor field (bits [23:20]) for fence instructions
    pub succ: u8,

    /// FM field (bits [31:28]) for fence instructions
    pub fm: u8,
}

impl EncodedInstruction {
    /// Parse all possible fields from a 32-bit instruction
    pub fn new(raw: u32) -> Self {
        /*
        Below are the six common instruction formats that all 32-bit
        instructions fit into.

        Note: The same variable is always in the same bit position
        i.e. rd is always bits 7 to 11 if it is present.

        EncodedInstruction can be seen as a union of all of the types,
        we then choose the appropriate fields based on the opcode in the decoding
        procedure.

        R-type | funct7 |  rs2 |  rs1 | funct3 |   rd  | opcode |
               | 31-25  |24-20 |19-15 | 14-12  | 11-7  | 6-0    |
        --------------------------------------------------------

        I-type |   imm[11:0]    |  rs1 | funct3 |   rd  | opcode |
               |   31-20        |19-15 | 14-12  | 11-7  | 6-0    |
        --------------------------------------------------------

        S-type | imm[11:5] |  rs2 |  rs1 | funct3 | imm[4:0] | opcode |
               | 31-25     |24-20 |19-15 | 14-12  | 11-7     | 6-0    |
        --------------------------------------------------------------

        B-type | imm[12] | imm[10:5] |  rs2 |  rs1 | funct3 | imm[4:1|11] | opcode |
               |   31    | 30-25     |24-20 |19-15 | 14-12  | 11-7        | 6-0    |
        ---------------------------------------------------------------------------

        U-type |                imm[31:12]                 |   rd  | opcode |
               |                31-12                      | 11-7  | 6-0    |
        --------------------------------------------------------------------

        J-type | imm[20] | imm[10:1] | imm[11] | imm[19:12] |   rd  | opcode |
               |   31    | 30-21     |   20    | 19-12      | 11-7  | 6-0    |
        --------------------------------------------------------------------
        */

        // Opcode is always the first 7 bits
        let opcode_raw = (raw & MASK7) as u8;
        // rd is always the next 5 bits
        let rd = ((raw >> 7) & MASK5) as u8;
        // funct3 is always the next 3 bits
        let funct3 = ((raw >> 12) & MASK3) as u8;
        // rs1 is always the next 5 bits
        let rs1 = ((raw >> 15) & MASK5) as u8;
        // rs2 is always the next 5 bits
        let rs2 = ((raw >> 20) & MASK5) as u8;
        // funct7 is always the next 7 bits
        let funct7 = ((raw >> 25) & MASK7) as u8;

        let opcode = Opcode::from_bits(opcode_raw);

        // Extract all possible immediate formats
        let i_immediate = Self::extract_i_immediate(raw);
        let s_immediate = Self::extract_s_immediate(raw);
        let b_immediate = Self::extract_b_immediate(raw);
        let u_immediate = Self::extract_u_immediate(raw);
        let j_immediate = Self::extract_j_immediate(raw);

        // Extract other specialized fields
        let csr = ((raw >> 20) & MASK12) as u16; // 12-bit CSR address -- note, no sign extension here for csr address
        let shamt32 = ((raw >> 20) & MASK5) as u8; // 5-bit shift amount for RV32I
        let shamt64 = ((raw >> 20) & MASK6) as u8; // 6-bit shift amount for RV64I
        let aq = ((raw >> 26) & MASK1) != 0;
        let rl = ((raw >> 25) & MASK1) != 0;
        let funct5 = ((raw >> 27) & MASK5) as u8;
        let pred = ((raw >> 24) & MASK4) as u8;
        let succ = ((raw >> 20) & MASK4) as u8;
        let fm = ((raw >> 28) & MASK4) as u8;

        Self {
            raw,
            opcode_raw,
            opcode,
            rd,
            funct3,
            rs1,
            rs2,
            funct7,
            i_immediate,
            s_immediate,
            b_immediate,
            u_immediate,
            j_immediate,
            csr,
            shamt32,
            shamt64,
            aq,
            rl,
            funct5,
            pred,
            succ,
            fm,
        }
    }

    /// Extract I-type immediate (12-bit, sign-extended)
    fn extract_i_immediate(raw: u32) -> i32 {
        let imm = (raw >> 20) & MASK12;

        // sign-extend from 12 bits
        ((imm as i32) << 20) >> 20
    }

    /// Extract S-type immediate (12-bit split, sign-extended)
    fn extract_s_immediate(raw: u32) -> i32 {
        let imm11_5 = ((raw >> 25) & MASK7) << 5;
        let imm4_0 = (raw >> 7) & MASK5;

        let imm = imm11_5 | imm4_0;

        // sign-extend from 12 bits
        ((imm as i32) << 20) >> 20
    }

    /// Extract B-type immediate (13-bit branch offset, sign-extended)
    fn extract_b_immediate(raw: u32) -> i32 {
        let imm12 = ((raw >> 31) & MASK1) << 12;
        let imm10_5 = ((raw >> 25) & MASK6) << 5;
        let imm4_1 = ((raw >> 8) & MASK4) << 1;
        let imm11 = ((raw >> 7) & MASK1) << 11;

        let imm = imm12 | imm11 | imm10_5 | imm4_1;

        // sign-extend from 13 bits
        ((imm as i32) << 19) >> 19
    }

    /// Extract U-type immediate (20-bit immediate value)
    fn extract_u_immediate(raw: u32) -> i32 {
        (raw >> 12) as i32
    }

    /// Extract J-type immediate (21-bit jump offset, sign-extended)  
    fn extract_j_immediate(raw: u32) -> i32 {
        let imm20 = ((raw >> 31) & MASK1) << 20;
        let imm10_1 = ((raw >> 21) & MASK10) << 1;
        let imm11 = ((raw >> 20) & MASK1) << 11;
        let imm19_12 = ((raw >> 12) & MASK8) << 12;

        let imm = imm20 | imm19_12 | imm11 | imm10_1;

        // sign-extend from 21 bits
        ((imm as i32) << 11) >> 11
    }

    /// Get the instruction format based on opcode
    /// TODO: Del this is only needed for Documentation and possibly tests
    /// TODO: so we can delete it and just have comments ontop of opcode for example
    /// TODO: THis would mean we no longer need InstructionFormat struct
    pub fn format(&self) -> Option<InstructionFormat> {
        match self.opcode? {
            Opcode::Op | Opcode::Op32 => Some(InstructionFormat::R),
            Opcode::Load
            | Opcode::OpImm
            | Opcode::OpImm32
            | Opcode::Jalr
            | Opcode::MiscMem
            | Opcode::System => Some(InstructionFormat::I),
            Opcode::Store => Some(InstructionFormat::S),
            Opcode::Branch => Some(InstructionFormat::B),
            Opcode::Lui | Opcode::Auipc => Some(InstructionFormat::U),
            Opcode::Jal => Some(InstructionFormat::J),
            Opcode::Amo => Some(InstructionFormat::R), // A-type uses R-type format base
        }
    }
}

/// Decode LOAD instructions
///
/// Uses standard I-type format (see InstructionFormat::I)
fn decode_load_instruction(
    encoded: &EncodedInstruction,
    target: &Target,
) -> Result<Instruction, DecodeError> {
    let rd = encoded.rd;
    let rs1 = encoded.rs1;
    let offset = encoded.i_immediate;

    match encoded.funct3 {
        0b000 => Ok(Instruction::LB { rd, rs1, offset }),
        0b001 => Ok(Instruction::LH { rd, rs1, offset }),
        0b010 => Ok(Instruction::LW { rd, rs1, offset }),
        0b011 => {
            if target.supports_extension(Extension::RV64I) {
                Ok(Instruction::LD { rd, rs1, offset })
            } else {
                Err(DecodeError::InvalidFormat)
            }
        }
        0b100 => Ok(Instruction::LBU { rd, rs1, offset }),
        0b101 => Ok(Instruction::LHU { rd, rs1, offset }),
        0b110 => {
            if target.supports_extension(Extension::RV64I) {
                Ok(Instruction::LWU { rd, rs1, offset })
            } else {
                Err(DecodeError::InvalidFormat)
            }
        }
        _ => Err(DecodeError::InvalidFormat),
    }
}

/// Decode STORE instructions
///
/// Uses standard S-type format (see InstructionFormat::S)
fn decode_store_instruction(
    encoded: &EncodedInstruction,
    target: &Target,
) -> Result<Instruction, DecodeError> {
    let rs1 = encoded.rs1;
    let rs2 = encoded.rs2;
    let offset = encoded.s_immediate;

    match encoded.funct3 {
        0b000 => Ok(Instruction::SB { rs1, rs2, offset }),
        0b001 => Ok(Instruction::SH { rs1, rs2, offset }),
        0b010 => Ok(Instruction::SW { rs1, rs2, offset }),
        0b011 => {
            if target.supports_extension(Extension::RV64I) {
                Ok(Instruction::SD { rs1, rs2, offset })
            } else {
                Err(DecodeError::InvalidFormat)
            }
        }
        _ => Err(DecodeError::InvalidFormat),
    }
}

/// Decode OP-IMM instructions (addi, slti, sltiu, xori, ori, andi, slli, srli, srai)
/// Uses standard I-type format (see InstructionFormat::I)
///
/// **Special handling for shift instructions (SLLI, SRLI, SRAI):**
/// - `shamt` (shift amount) comes from bits [25:20] of the I-type immediate field
/// - RV32I: uses 5-bit shamt (bits [24:20]), bit [25] must be 0
/// - RV64I: uses 6-bit shamt (bits [25:20])
/// - `funct7` field (bits [31:25]) distinguishes SRLI (0000000) vs SRAI (0100000)
fn decode_op_imm_instruction(
    encoded: &EncodedInstruction,
    target: &Target,
) -> Result<Instruction, DecodeError> {
    let rd = encoded.rd;
    let rs1 = encoded.rs1;
    let imm = encoded.i_immediate;
    // I-type doesn't use funct7, but we just re-use it to get top 7 bits
    // Could just as well shift on the immediate
    let funct7 = encoded.funct7;

    let is_rv64 = target.supports_extension(Extension::RV64I);
    let shamt = if is_rv64 { encoded.shamt64 } else { encoded.shamt32 };
    // imm upper bits used for validation
    let imm_hi6 = ((funct7 as u32 >> 1) & MASK6) as u8; // imm[11:6]

    match encoded.funct3 {
        0b000 => Ok(Instruction::ADDI { rd, rs1, imm }),
        0b001 => {
            // SLLI: check reserved upper immediate bits
            if is_rv64 {
                if imm_hi6 != 0 {
                    return Err(DecodeError::InvalidFormat);
                }
            } else if funct7 != 0 {
                return Err(DecodeError::InvalidFormat);
            }
            Ok(Instruction::SLLI { rd, rs1, shamt })
        }
        0b010 => Ok(Instruction::SLTI { rd, rs1, imm }),
        0b011 => Ok(Instruction::SLTIU { rd, rs1, imm }),
        0b100 => Ok(Instruction::XORI { rd, rs1, imm }),
        0b101 => {
            if is_rv64 {
                match imm_hi6 {
                    0b000000 => Ok(Instruction::SRLI { rd, rs1, shamt }),
                    0b01_0000 => Ok(Instruction::SRAI { rd, rs1, shamt }),
                    _ => Err(DecodeError::InvalidFormat),
                }
            } else {
                match funct7 {
                    0b000_0000 => Ok(Instruction::SRLI { rd, rs1, shamt }),
                    0b010_0000 => Ok(Instruction::SRAI { rd, rs1, shamt }),
                    _ => Err(DecodeError::InvalidFormat),
                }
            }
        }
        0b110 => Ok(Instruction::ORI { rd, rs1, imm }),
        0b111 => Ok(Instruction::ANDI { rd, rs1, imm }),
        _ => Err(DecodeError::InvalidFormat),
    }
}

/// Decode OP instructions (register-register operations)
///
/// Uses standard R-type format (see InstructionFormat::R)
fn decode_op_instruction(
    encoded: &EncodedInstruction,
    target: &Target,
) -> Result<Instruction, DecodeError> {
    let rd = encoded.rd;
    let rs1 = encoded.rs1;
    let rs2 = encoded.rs2;
    let has_m_ext = target.supports_extension(Extension::RV32M);

    match (encoded.funct3, encoded.funct7) {
        // Base RV32I arithmetic
        (0b000, 0b000_0000) => Ok(Instruction::ADD { rd, rs1, rs2 }),
        (0b000, 0b010_0000) => Ok(Instruction::SUB { rd, rs1, rs2 }),
        (0b001, 0b000_0000) => Ok(Instruction::SLL { rd, rs1, rs2 }),
        (0b010, 0b000_0000) => Ok(Instruction::SLT { rd, rs1, rs2 }),
        (0b011, 0b000_0000) => Ok(Instruction::SLTU { rd, rs1, rs2 }),
        (0b100, 0b000_0000) => Ok(Instruction::XOR { rd, rs1, rs2 }),
        (0b101, 0b000_0000) => Ok(Instruction::SRL { rd, rs1, rs2 }),
        (0b101, 0b010_0000) => Ok(Instruction::SRA { rd, rs1, rs2 }),
        (0b110, 0b000_0000) => Ok(Instruction::OR { rd, rs1, rs2 }),
        (0b111, 0b000_0000) => Ok(Instruction::AND { rd, rs1, rs2 }),

        // RV32M multiply/divide extension
        (0b000, 0b000_0001) if has_m_ext => Ok(Instruction::MUL { rd, rs1, rs2 }),
        (0b001, 0b000_0001) if has_m_ext => Ok(Instruction::MULH { rd, rs1, rs2 }),
        (0b010, 0b000_0001) if has_m_ext => Ok(Instruction::MULHSU { rd, rs1, rs2 }),
        (0b011, 0b000_0001) if has_m_ext => Ok(Instruction::MULHU { rd, rs1, rs2 }),
        (0b100, 0b000_0001) if has_m_ext => Ok(Instruction::DIV { rd, rs1, rs2 }),
        (0b101, 0b000_0001) if has_m_ext => Ok(Instruction::DIVU { rd, rs1, rs2 }),
        (0b110, 0b000_0001) if has_m_ext => Ok(Instruction::REM { rd, rs1, rs2 }),
        (0b111, 0b000_0001) if has_m_ext => Ok(Instruction::REMU { rd, rs1, rs2 }),

        _ => Err(DecodeError::InvalidFormat),
    }
}

/// Decode BRANCH instructions
///
/// Uses standard B-type format (see InstructionFormat::B)
fn decode_branch_instruction(encoded: &EncodedInstruction) -> Result<Instruction, DecodeError> {
    let rs1 = encoded.rs1;
    let rs2 = encoded.rs2;
    let offset = encoded.b_immediate;

    match encoded.funct3 {
        0b000 => Ok(Instruction::BEQ { rs1, rs2, offset }),
        0b001 => Ok(Instruction::BNE { rs1, rs2, offset }),
        0b100 => Ok(Instruction::BLT { rs1, rs2, offset }),
        0b101 => Ok(Instruction::BGE { rs1, rs2, offset }),
        0b110 => Ok(Instruction::BLTU { rs1, rs2, offset }),
        0b111 => Ok(Instruction::BGEU { rs1, rs2, offset }),
        _ => Err(DecodeError::InvalidFormat),
    }
}

/// Decode JAL instruction
///
/// Uses standard J-type format (see InstructionFormat::J)
fn decode_jal_instruction(encoded: &EncodedInstruction) -> Result<Instruction, DecodeError> {
    let rd = encoded.rd;
    let offset = encoded.j_immediate;
    Ok(Instruction::JAL { rd, offset })
}

/// Decode JALR instruction
///
/// Uses standard I-type format (see InstructionFormat::I)  
fn decode_jalr_instruction(encoded: &EncodedInstruction) -> Result<Instruction, DecodeError> {
    if encoded.funct3 != 0b000 {
        return Err(DecodeError::InvalidFormat);
    }
    let rd = encoded.rd;
    let rs1 = encoded.rs1;
    let offset = encoded.i_immediate;
    Ok(Instruction::JALR { rd, rs1, offset })
}

/// Decode LUI instruction
///
/// Uses standard U-type format (see InstructionFormat::U)
fn decode_lui_instruction(encoded: &EncodedInstruction) -> Result<Instruction, DecodeError> {
    let rd = encoded.rd;
    let imm = encoded.u_immediate;
    Ok(Instruction::LUI { rd, imm })
}

/// Decode AUIPC instruction
///
/// Uses standard U-type format (see InstructionFormat::U)
fn decode_auipc_instruction(encoded: &EncodedInstruction) -> Result<Instruction, DecodeError> {
    let rd = encoded.rd;
    let imm = encoded.u_immediate;
    Ok(Instruction::AUIPC { rd, imm })
}

/// Decode SYSTEM instructions
///
/// Uses standard I-type format (see InstructionFormat::I)
fn decode_system_instruction(
    encoded: &EncodedInstruction,
    target: &Target,
) -> Result<Instruction, DecodeError> {
    let rd = encoded.rd;
    let rs1 = encoded.rs1;
    let csr = encoded.csr;
    let uimm = encoded.rs1; // For CSR immediate instructions, rs1 field contains immediate

    match encoded.funct3 {
        0b000 => {
            // ECALL/EBREAK distinguished by I-type immediate field
            match encoded.i_immediate {
                0 => {
                    if rd != 0 || rs1 != 0 {
                        return Err(DecodeError::InvalidFormat);
                    }
                    Ok(Instruction::ECALL)
                }
                1 => {
                    if rd != 0 || rs1 != 0 {
                        return Err(DecodeError::InvalidFormat);
                    }
                    Ok(Instruction::EBREAK)
                }
                _ => Err(DecodeError::InvalidFormat),
            }
        }
        0b001 | 0b010 | 0b011 | 0b101 | 0b110 | 0b111 => {
            // CSR instructions require Zicsr
            if !target.supports_extension(Extension::Zicsr) {
                return Err(DecodeError::UnsupportedExtension("Zicsr".to_string()));
            }
            match encoded.funct3 {
                0b001 => Ok(Instruction::CSRRW { rd, rs1, csr }),
                0b010 => Ok(Instruction::CSRRS { rd, rs1, csr }),
                0b011 => Ok(Instruction::CSRRC { rd, rs1, csr }),
                0b101 => Ok(Instruction::CSRRWI { rd, uimm, csr }),
                0b110 => Ok(Instruction::CSRRSI { rd, uimm, csr }),
                0b111 => Ok(Instruction::CSRRCI { rd, uimm, csr }),
                _ => unreachable!("`funct3` should be encoded with 3 bits"),
            }
        }
        _ => Err(DecodeError::InvalidFormat),
    }
}

/// Decode FENCE instructions
///
/// Uses standard I-type format (see InstructionFormat::I)
///
/// The docs also note how fence specific information is encoded
/// in the I-type.
fn decode_fence_instruction(
    encoded: &EncodedInstruction,
    target: &Target,
) -> Result<Instruction, DecodeError> {
    let pred = encoded.pred;
    let succ = encoded.succ;
    let fm = encoded.fm;
    // TODO: check funct12 -- possibly parse funct12 for readability
    match encoded.funct3 {
        0b000 => {
            // rd and rs1 must be zero
            if encoded.rd != 0 || encoded.rs1 != 0 {
                return Err(DecodeError::InvalidFormat);
            }
            if fm != 0 {
                return Err(DecodeError::InvalidFormat);
            }
            Ok(Instruction::FENCE { pred, succ })
        }
        0b001 => {
            // rd and rs1 must be zero
            if encoded.rd != 0 || encoded.rs1 != 0 {
                return Err(DecodeError::InvalidFormat);
            }
            if !target.supports_extension(Extension::Zifencei) {
                return Err(DecodeError::UnsupportedExtension("Zifencei".to_string()));
            }
            Ok(Instruction::FENCE_I)
        }
        _ => Err(DecodeError::InvalidFormat),
    }
}

/// Decode OP-IMM-32 instructions (RV64I word immediate operations)
///
/// Uses standard I-type format (see InstructionFormat::I)
///
/// - `shamt` (shift amount) is encoded in the immediate field of the I-type.
///
/// Note: Even though these instructions are defined for RV64I.
/// For the shift related instructions, we only use a 5-bit `shamt`
/// because it is operating on a 32-bit word.
fn decode_op_imm_32_instruction(
    encoded: &EncodedInstruction,
    target: &Target,
) -> Result<Instruction, DecodeError> {
    if !target.supports_extension(Extension::RV64I) {
        return Err(DecodeError::UnsupportedExtension("RV64I".to_string()));
    }

    match encoded.funct3 {
        0 => Ok(Instruction::ADDIW { rd: encoded.rd, rs1: encoded.rs1, imm: encoded.i_immediate }),
        1 => {
            if encoded.funct7 == 0 {
                let shamt = encoded.shamt32;
                Ok(Instruction::SLLIW { rd: encoded.rd, rs1: encoded.rs1, shamt })
            } else {
                Err(DecodeError::InvalidFormat)
            }
        }
        5 => {
            let shamt = encoded.shamt32;
            match encoded.funct7 {
                0 => Ok(Instruction::SRLIW { rd: encoded.rd, rs1: encoded.rs1, shamt }),
                32 => Ok(Instruction::SRAIW { rd: encoded.rd, rs1: encoded.rs1, shamt }),
                _ => Err(DecodeError::InvalidFormat),
            }
        }
        _ => Err(DecodeError::InvalidFormat),
    }
}

/// Decode OP-32 instructions (RV64I word register operations)
///
/// Uses standard R-type format (see InstructionFormat::R)  
fn decode_op_32_instruction(
    encoded: &EncodedInstruction,
    target: &Target,
) -> Result<Instruction, DecodeError> {
    if !target.supports_extension(Extension::RV64I) {
        return Err(DecodeError::UnsupportedExtension("RV64I".to_string()));
    }

    let rd = encoded.rd;
    let rs1 = encoded.rs1;
    let rs2 = encoded.rs2;
    let has_m_ext = target.supports_extension(Extension::RV64M);

    match (encoded.funct3, encoded.funct7) {
        // Base RV64I word operations
        (0b000, 0b000_0000) => Ok(Instruction::ADDW { rd, rs1, rs2 }),
        (0b000, 0b010_0000) => Ok(Instruction::SUBW { rd, rs1, rs2 }),
        (0b001, 0b000_0000) => Ok(Instruction::SLLW { rd, rs1, rs2 }),
        (0b101, 0b000_0000) => Ok(Instruction::SRLW { rd, rs1, rs2 }),
        (0b101, 0b010_0000) => Ok(Instruction::SRAW { rd, rs1, rs2 }),

        // RV64M word multiply/divide extension
        (0b000, 0b000_0001) if has_m_ext => Ok(Instruction::MULW { rd, rs1, rs2 }),
        (0b100, 0b000_0001) if has_m_ext => Ok(Instruction::DIVW { rd, rs1, rs2 }),
        (0b101, 0b000_0001) if has_m_ext => Ok(Instruction::DIVUW { rd, rs1, rs2 }),
        (0b110, 0b000_0001) if has_m_ext => Ok(Instruction::REMW { rd, rs1, rs2 }),
        (0b111, 0b000_0001) if has_m_ext => Ok(Instruction::REMUW { rd, rs1, rs2 }),

        _ => Err(DecodeError::InvalidFormat),
    }
}

/// Decode AMO (atomic) instructions
/// Uses standard R-type format (see InstructionFormat::R)
///
/// The docs also note how atomic specific information is encoded
/// in the R-type.
fn decode_amo_instruction(
    encoded: &EncodedInstruction,
    target: &Target,
) -> Result<Instruction, DecodeError> {
    let has_rv32a = target.supports_extension(Extension::RV32A);
    let has_rv64a = target.supports_extension(Extension::RV64A);

    if !has_rv32a && !has_rv64a {
        return Err(DecodeError::UnsupportedExtension("Atomic extension required".to_string()));
    }

    let rd = encoded.rd;
    let rs1 = encoded.rs1;
    let rs2 = encoded.rs2;
    let aq = encoded.aq;
    let rl = encoded.rl;

    match (encoded.funct3, encoded.funct5) {
        // Word atomic operations (32-bit) - requires RV32A
        (0b010, 0b00010) if has_rv32a => Ok(Instruction::LR_W { rd, rs1, aq, rl }),
        (0b010, 0b00011) if has_rv32a => Ok(Instruction::SC_W { rd, rs1, rs2, aq, rl }),
        (0b010, 0b00001) if has_rv32a => Ok(Instruction::AMOSWAP_W { rd, rs1, rs2, aq, rl }),
        (0b010, 0b00000) if has_rv32a => Ok(Instruction::AMOADD_W { rd, rs1, rs2, aq, rl }),
        (0b010, 0b00100) if has_rv32a => Ok(Instruction::AMOXOR_W { rd, rs1, rs2, aq, rl }),
        (0b010, 0b01100) if has_rv32a => Ok(Instruction::AMOAND_W { rd, rs1, rs2, aq, rl }),
        (0b010, 0b01000) if has_rv32a => Ok(Instruction::AMOOR_W { rd, rs1, rs2, aq, rl }),
        (0b010, 0b10000) if has_rv32a => Ok(Instruction::AMOMIN_W { rd, rs1, rs2, aq, rl }),
        (0b010, 0b10100) if has_rv32a => Ok(Instruction::AMOMAX_W { rd, rs1, rs2, aq, rl }),
        (0b010, 0b11000) if has_rv32a => Ok(Instruction::AMOMINU_W { rd, rs1, rs2, aq, rl }),
        (0b010, 0b11100) if has_rv32a => Ok(Instruction::AMOMAXU_W { rd, rs1, rs2, aq, rl }),

        // Doubleword atomic operations (64-bit) - requires RV64A
        (0b011, 0b00010) if has_rv64a => Ok(Instruction::LR_D { rd, rs1, aq, rl }),
        (0b011, 0b00011) if has_rv64a => Ok(Instruction::SC_D { rd, rs1, rs2, aq, rl }),
        (0b011, 0b00001) if has_rv64a => Ok(Instruction::AMOSWAP_D { rd, rs1, rs2, aq, rl }),
        (0b011, 0b00000) if has_rv64a => Ok(Instruction::AMOADD_D { rd, rs1, rs2, aq, rl }),
        (0b011, 0b00100) if has_rv64a => Ok(Instruction::AMOXOR_D { rd, rs1, rs2, aq, rl }),
        (0b011, 0b01100) if has_rv64a => Ok(Instruction::AMOAND_D { rd, rs1, rs2, aq, rl }),
        (0b011, 0b01000) if has_rv64a => Ok(Instruction::AMOOR_D { rd, rs1, rs2, aq, rl }),
        (0b011, 0b10000) if has_rv64a => Ok(Instruction::AMOMIN_D { rd, rs1, rs2, aq, rl }),
        (0b011, 0b10100) if has_rv64a => Ok(Instruction::AMOMAX_D { rd, rs1, rs2, aq, rl }),
        (0b011, 0b11000) if has_rv64a => Ok(Instruction::AMOMINU_D { rd, rs1, rs2, aq, rl }),
        (0b011, 0b11100) if has_rv64a => Ok(Instruction::AMOMAXU_D { rd, rs1, rs2, aq, rl }),

        _ => Err(DecodeError::InvalidFormat),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_decode_addi() {
        let target = Target::new();
        // addi x1, x0, 42 = 0x02A00093
        let result = decode_standard_instruction(0x02A00093, &target).unwrap();

        match result {
            Instruction::ADDI { rd, rs1, imm } => {
                assert_eq!(rd, 1);
                assert_eq!(rs1, 0);
                assert_eq!(imm, 42);
            }
            _ => panic!("Expected ADDI instruction"),
        }
    }

    #[test]
    fn test_decode_add() {
        let target = Target::new();
        // add x3, x1, x2 = 0x002081B3
        let result = decode_standard_instruction(0x002081B3, &target).unwrap();

        match result {
            Instruction::ADD { rd, rs1, rs2 } => {
                assert_eq!(rd, 3);
                assert_eq!(rs1, 1);
                assert_eq!(rs2, 2);
            }
            _ => panic!("Expected ADD instruction"),
        }
    }

    #[test]
    fn test_decode_illegal() {
        let target = Target::new();
        let result = decode_standard_instruction(0x00000000, &target).unwrap();
        assert!(result.is_illegal());
    }

    #[test]
    fn test_rv64_extension_required() {
        let rv64_target = Target::new().with_64bit();

        // ADDIW requires RV64I - with assert, we only test the working case
        let addiw_bits = 0x0010009B; // addiw x1, x0, 1

        let rv64_result = decode_standard_instruction(addiw_bits, &rv64_target);
        assert!(rv64_result.is_ok());
    }

    #[test]
    fn test_op32_opimm32_invalid_on_rv32() {
        let rv32_target = Target::new();
        // addiw x1, x0, 1 (0011011)
        let addiw_bits = 0x0010009B;
        let res1 = decode_standard_instruction(addiw_bits, &rv32_target);
        assert!(res1.is_err());

        // addw x1, x0, x0 (0111011)
        let addw_bits = 0x0000103B;
        let res2 = decode_standard_instruction(addw_bits, &rv32_target);
        assert!(res2.is_err());
    }

    #[test]
    fn test_rv64_srli_srai_imm_hi6_patterns() {
        let target = Target::new().with_64bit();

        // SRLI rd=x1, rs1=x1, shamt=33 (0b100001)
        // RV64: imm_hi6 must be 000000, shamt in imm[5:0]
        let rd = 1u32;
        let rs1 = 1u32;
        let shamt = 33u32; // uses imm[5]=1 to ensure allowed
        let imm_hi6_srli = 0u32; // 000000
        let imm_srli = (imm_hi6_srli << 6) | shamt;
        let srli_bits = (imm_srli << 20) | (rs1 << 15) | (0b101 << 12) | (rd << 7) | 0x13;
        let res_srli = decode_standard_instruction(srli_bits, &target).unwrap();
        match res_srli {
            Instruction::SRLI { rd: rd2, rs1: rs12, shamt: s } => {
                assert_eq!(rd2, 1);
                assert_eq!(rs12, 1);
                assert_eq!(s, shamt as u8);
            }
            _ => panic!("Expected SRLI"),
        }

        // SRAI rd=x1, rs1=x1, shamt=33 (0b100001)
        // RV64: imm_hi6 must be 010000
        let imm_hi6_srai = 0b010000u32; // 16
        let imm_srai = (imm_hi6_srai << 6) | shamt;
        let srai_bits = (imm_srai << 20) | (rs1 << 15) | (0b101 << 12) | (rd << 7) | 0x13;
        let res_srai = decode_standard_instruction(srai_bits, &target).unwrap();
        match res_srai {
            Instruction::SRAI { rd: rd2, rs1: rs12, shamt: s } => {
                assert_eq!(rd2, 1);
                assert_eq!(rs12, 1);
                assert_eq!(s, shamt as u8);
            }
            _ => panic!("Expected SRAI"),
        }

        // Invalid SRLI: imm_hi6 non-zero should be rejected
        let imm_hi6_bad = 0b000001u32;
        let imm_bad = (imm_hi6_bad << 6) | shamt;
        let srli_bad_bits = (imm_bad << 20) | (rs1 << 15) | (0b101 << 12) | (rd << 7) | 0x13;
        assert!(decode_standard_instruction(srli_bad_bits, &target).is_err());

        // Invalid SRAI: wrong imm_hi6 pattern should be rejected
        let imm_hi6_wrong = 0b001000u32;
        let imm_wrong = (imm_hi6_wrong << 6) | shamt;
        let srai_bad_bits = (imm_wrong << 20) | (rs1 << 15) | (0b101 << 12) | (rd << 7) | 0x13;
        assert!(decode_standard_instruction(srai_bad_bits, &target).is_err());
    }

    #[test]
    fn test_ld_invalid_on_rv32() {
        let target = Target::new(); // RV32I
                                    // ld x1, 0(x0): opcode=0000011, funct3=011, rd=1, rs1=0, imm=0
        let bits = (0 << 20) | (0 << 15) | (0b011 << 12) | (1 << 7) | 0x03;
        let res = decode_standard_instruction(bits, &target);
        assert!(res.is_err());
    }

    #[test]
    fn test_sd_invalid_on_rv32() {
        let target = Target::new(); // RV32I
                                    // sd x1, 0(x0): opcode=0100011, funct3=011, rs2=1, rs1=0, imm=0
        let opcode = 0x23u32;
        let bits = (0 << 25) | (1 << 20) | (0 << 15) | (0b011 << 12) | (0 << 7) | opcode;
        let res = decode_standard_instruction(bits, &target);
        assert!(res.is_err());
    }

    #[test]
    fn test_slli_reserved_bits_rv32_invalid() {
        let target = Target::new(); // RV32I
                                    // slli x1, x1, shamt=1 but set imm[5]=1 (bit 25), which is reserved in RV32
                                    // opcode=0010011, funct3=001, rd=1, rs1=1, imm=(1<<5)|1
        let imm = (1 << 5) | 1; // 0b100001
        let bits = (imm << 20) | (1 << 15) | (0b001 << 12) | (1 << 7) | 0x13;
        let res = decode_standard_instruction(bits, &target);
        assert!(res.is_err());
    }

    #[test]
    fn test_jalr_invalid_funct3() {
        let target = Target::new();
        // jalr with funct3 != 000 should be invalid: funct3=001
        let bits = (0 << 20) | (1 << 15) | (0b001 << 12) | (1 << 7) | 0x67;
        let res = decode_standard_instruction(bits, &target);
        assert!(res.is_err());
    }

    #[test]
    fn test_fence_i_requires_zifencei() {
        let target = Target::new(); // no Zifencei
                                    // fence.i encoding: funct3=001, opcode=0001111
        let bits = (0 << 20) | (0 << 15) | (0b001 << 12) | (0 << 7) | 0x0F;
        let res = decode_standard_instruction(bits, &target);
        assert!(res.is_err());
    }

    #[test]
    fn test_csr_requires_zicsr() {
        let target = Target::new(); // no Zicsr
                                    // csrrw x1, csr=0, x1: funct3=001, opcode=1110011
        let bits = (0 << 20) | (1 << 15) | (0b001 << 12) | (1 << 7) | 0x73;
        let res = decode_standard_instruction(bits, &target);
        assert!(res.is_err());
    }

    #[test]
    fn test_fence_requires_zero_regs() {
        let target = Target::new();
        // fence with rd!=0 (invalid)
        let bits_bad_rd = (0 << 24) | (0 << 20) | (0 << 15) | (0b000 << 12) | (1 << 7) | 0x0F;
        let res_bad_rd = decode_standard_instruction(bits_bad_rd, &target);
        assert!(res_bad_rd.is_err());

        // fence with rs1!=0 (invalid)
        let bits_bad_rs1 = (0 << 24) | (0 << 20) | (1 << 15) | (0b000 << 12) | (0 << 7) | 0x0F;
        let res_bad_rs1 = decode_standard_instruction(bits_bad_rs1, &target);
        assert!(res_bad_rs1.is_err());

        // valid fence with zero regs, fm=0
        let bits_ok = (0 << 24) | (0 << 20) | (0 << 15) | (0b000 << 12) | (0 << 7) | 0x0F;
        let res_ok = decode_standard_instruction(bits_ok, &target);
        assert!(res_ok.is_ok());
    }

    #[test]
    fn test_fence_i_requires_zero_regs_and_ext() {
        // Enable Zifencei for this test to check regs enforcement
        let target = Target::new().with_zifencei();
        // fence.i with rd!=0 (invalid)
        let bits_bad_rd = (0 << 20) | (0 << 15) | (0b001 << 12) | (1 << 7) | 0x0F;
        let res_bad_rd = decode_standard_instruction(bits_bad_rd, &target);
        assert!(res_bad_rd.is_err());

        // fence.i with rs1!=0 (invalid)
        let bits_bad_rs1 = (0 << 20) | (1 << 15) | (0b001 << 12) | (0 << 7) | 0x0F;
        let res_bad_rs1 = decode_standard_instruction(bits_bad_rs1, &target);
        assert!(res_bad_rs1.is_err());

        // valid fence.i with zero regs
        let bits_ok = (0 << 20) | (0 << 15) | (0b001 << 12) | (0 << 7) | 0x0F;
        let res_ok = decode_standard_instruction(bits_ok, &target);
        assert!(res_ok.is_ok());
    }

    #[test]
    fn test_ecall_ebreak_require_zero_regs() {
        let target = Target::new();
        // ecall canonical: imm=0, rs1=0, rd=0
        let ecall_ok = (0 << 20) | (0 << 15) | (0b000 << 12) | (0 << 7) | 0x73;
        let res_ok = decode_standard_instruction(ecall_ok, &target);
        assert!(res_ok.is_ok());

        // ecall with rd != 0 -> invalid
        let ecall_bad_rd = (0 << 20) | (0 << 15) | (0b000 << 12) | (1 << 7) | 0x73;
        let res_bad_rd = decode_standard_instruction(ecall_bad_rd, &target);
        assert!(res_bad_rd.is_err());

        // ebreak canonical: imm=1, rs1=0, rd=0
        let ebreak_ok = (1 << 20) | (0 << 15) | (0b000 << 12) | (0 << 7) | 0x73;
        let res_ok2 = decode_standard_instruction(ebreak_ok, &target);
        assert!(res_ok2.is_ok());

        // ebreak with rs1 != 0 -> invalid
        let ebreak_bad_rs1 = (1 << 20) | (1 << 15) | (0b000 << 12) | (0 << 7) | 0x73;
        let res_bad_rs1 = decode_standard_instruction(ebreak_bad_rs1, &target);
        assert!(res_bad_rs1.is_err());
    }
}
