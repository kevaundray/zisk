//! RISC-V instruction types and definitions

pub mod formats;
pub mod opcodes;

pub use formats::*;
pub use opcodes::*;

use std::fmt;

/// A fully decoded RISC-V instruction with type-safe format-specific fields
/// TODO: Maybe deduplicate the repeated fields
#[derive(Debug, Clone, PartialEq)]
pub enum DecodedInstruction {
    /// R-type: register-register operations (add, sub, sll, etc.)
    RType {
        raw: u32,
        opcode: Opcode,
        mnemonic: String,
        rd: u8,
        rs1: u8,
        rs2: u8,
        funct3: u8,
        funct7: u8,
    },

    /// I-type: immediate operations and loads (addi, lw, jalr, etc.)
    IType {
        raw: u32,
        opcode: Opcode,
        mnemonic: String,
        rd: u8,
        rs1: u8,
        imm: i32,
        funct3: u8,
        funct7: u8, // For shift instructions
    },

    /// S-type: store operations (sw, sb, sh, sd)
    SType { raw: u32, opcode: Opcode, mnemonic: String, rs1: u8, rs2: u8, imm: i32, funct3: u8 },

    /// B-type: conditional branches (beq, bne, blt, etc.)
    BType { raw: u32, opcode: Opcode, mnemonic: String, rs1: u8, rs2: u8, imm: i32, funct3: u8 },

    /// U-type: upper immediate operations (lui, auipc)
    UType { raw: u32, opcode: Opcode, mnemonic: String, rd: u8, imm: i32 },

    /// J-type: unconditional jumps (jal)
    JType { raw: u32, opcode: Opcode, mnemonic: String, rd: u8, imm: i32 },

    /// A-type: atomic memory operations (lr.w, sc.w, amoswap.w, etc.)
    AType {
        raw: u32,
        opcode: Opcode,
        mnemonic: String,
        rd: u8,
        rs1: u8,
        rs2: u8,
        funct3: u8,
        funct5: u8,
        aq: bool,
        rl: bool,
    },

    /// F-type: fence operations (fence, fence.i)
    FType {
        raw: u32,
        opcode: Opcode,
        mnemonic: String,
        rd: u8,
        rs1: u8,
        funct3: u8,
        pred: u8,
        succ: u8,
    },

    /// System instructions (ecall, ebreak, csr operations)
    System { raw: u32, opcode: Opcode, mnemonic: String, rd: u8, rs1: u8, funct3: u8, csr: u32 },

    /// Illegal instruction (used for invalid/error conditions)
    Illegal,

    /// 16-bit compressed instruction with preserved info + 32-bit expansion
    Compressed {
        /// Original 16-bit instruction
        raw: u16,
        /// Compressed format type (CR, CI, CSS, etc.)
        compressed_format: CompressedFormat,
        /// Compressed instruction mnemonic (e.g. "c.addi", "c.lw")
        compressed_mnemonic: String,
        /// What this instruction expands to in 32-bit form
        expanded: Box<DecodedInstruction>,
    },
}

impl DecodedInstruction {
    /// Create a NOP instruction (addi x0, x0, 0)
    pub fn nop() -> Self {
        DecodedInstruction::IType {
            raw: 0x00000013, // addi x0, x0, 0
            opcode: Opcode::OpImm,
            mnemonic: "addi".to_string(),
            rd: 0,
            rs1: 0,
            imm: 0,
            funct3: 0,
            funct7: 0,
        }
    }

    /// Create an illegal instruction
    pub fn illegal() -> Self {
        DecodedInstruction::Illegal
    }
    
    /// Create a compressed illegal instruction (c.unimp)
    pub fn compressed_illegal() -> Self {
        DecodedInstruction::Compressed {
            raw: 0x0000,
            compressed_format: CompressedFormat::CIW, // Placeholder format for illegal
            compressed_mnemonic: "c.unimp".to_string(),
            expanded: Box::new(DecodedInstruction::illegal()),
        }
    }

    /// Check if this instruction is a NOP, returning its size if it is a NOP
    pub fn check_for_nop(&self) -> Option<usize> {
        let is_nop = match self {
            // Standard NOP: addi x0, x0, 0
            DecodedInstruction::IType {
                opcode: Opcode::OpImm,
                mnemonic,
                rd: 0,
                rs1: 0,
                imm: 0,
                funct3: 0,
                ..
            } => mnemonic == "addi",
            // Compressed NOP: c.nop (expands to standard NOP)
            DecodedInstruction::Compressed { expanded, .. } => expanded.check_for_nop().is_some(),
            _ => false,
        };
        
        if is_nop {
            Some(self.length_bytes() as usize)
        } else {
            None
        }
    }
    
    /// Check if this instruction is a NOP (both standard and compressed)
    pub fn is_nop(&self) -> bool {
        self.check_for_nop().is_some()
    }

    /// Check if this instruction is illegal, returning its size if illegal
    pub fn check_for_illegal(&self) -> Option<usize> {
        let is_illegal = match self {
            DecodedInstruction::Illegal => true,
            DecodedInstruction::Compressed { expanded, .. } => expanded.check_for_illegal().is_some(),
            _ => false,
        };
        
        if is_illegal {
            Some(self.length_bytes() as usize)
        } else {
            None
        }
    }
    
    /// Check if this instruction is illegal (convenience method)
    pub fn is_illegal(&self) -> bool {
        self.check_for_illegal().is_some()
    }
    

    /// Get the raw instruction word (32-bit for standard, 16-bit for compressed as u32)
    pub fn raw(&self) -> u32 {
        match self {
            DecodedInstruction::RType { raw, .. } => *raw,
            DecodedInstruction::IType { raw, .. } => *raw,
            DecodedInstruction::SType { raw, .. } => *raw,
            DecodedInstruction::BType { raw, .. } => *raw,
            DecodedInstruction::UType { raw, .. } => *raw,
            DecodedInstruction::JType { raw, .. } => *raw,
            DecodedInstruction::AType { raw, .. } => *raw,
            DecodedInstruction::FType { raw, .. } => *raw,
            DecodedInstruction::System { raw, .. } => *raw,
            DecodedInstruction::Illegal => 0x00000000,
            DecodedInstruction::Compressed { raw, .. } => *raw as u32,
        }
    }

    /// Get the raw 16-bit instruction for compressed instructions
    pub fn raw_compressed(&self) -> Option<u16> {
        match self {
            DecodedInstruction::Compressed { raw, .. } => Some(*raw),
            _ => None,
        }
    }

    /// Check if this is a compressed instruction
    pub fn is_compressed(&self) -> bool {
        matches!(self, DecodedInstruction::Compressed { .. })
    }

    /// Get the expanded 32-bit equivalent if this is a compressed instruction
    pub fn expanded(&self) -> Option<&DecodedInstruction> {
        match self {
            DecodedInstruction::Compressed { expanded, .. } => Some(expanded),
            _ => None,
        }
    }

    /// Get the instruction length in bytes (2 for compressed, 4 for standard)
    pub fn length_bytes(&self) -> u8 {
        match self {
            DecodedInstruction::Compressed { .. } => 2,
            _ => 4,
        }
    }

    /// Get the instruction opcode
    pub fn opcode(&self) -> Opcode {
        match self {
            DecodedInstruction::RType { opcode, .. }
            | DecodedInstruction::IType { opcode, .. }
            | DecodedInstruction::SType { opcode, .. }
            | DecodedInstruction::BType { opcode, .. }
            | DecodedInstruction::UType { opcode, .. }
            | DecodedInstruction::JType { opcode, .. }
            | DecodedInstruction::AType { opcode, .. }
            | DecodedInstruction::FType { opcode, .. }
            | DecodedInstruction::System { opcode, .. } => *opcode,
            DecodedInstruction::Illegal => Opcode::Illegal,
            DecodedInstruction::Compressed { expanded, .. } => expanded.opcode(),
        }
    }

    /// Get the instruction mnemonic (e.g., "add", "lw", "beq")
    pub fn mnemonic(&self) -> &str {
        match self {
            DecodedInstruction::RType { mnemonic, .. }
            | DecodedInstruction::IType { mnemonic, .. }
            | DecodedInstruction::SType { mnemonic, .. }
            | DecodedInstruction::BType { mnemonic, .. }
            | DecodedInstruction::UType { mnemonic, .. }
            | DecodedInstruction::JType { mnemonic, .. }
            | DecodedInstruction::AType { mnemonic, .. }
            | DecodedInstruction::FType { mnemonic, .. }
            | DecodedInstruction::System { mnemonic, .. } => mnemonic,
            DecodedInstruction::Illegal => "illegal",
            DecodedInstruction::Compressed { compressed_mnemonic, .. } => compressed_mnemonic,
        }
    }

    /// Get the instruction format
    pub fn format(&self) -> InstructionFormat {
        match self {
            DecodedInstruction::RType { .. } => InstructionFormat::R,
            DecodedInstruction::IType { .. } => InstructionFormat::I,
            DecodedInstruction::SType { .. } => InstructionFormat::S,
            DecodedInstruction::BType { .. } => InstructionFormat::B,
            DecodedInstruction::UType { .. } => InstructionFormat::U,
            DecodedInstruction::JType { .. } => InstructionFormat::J,
            DecodedInstruction::AType { .. } => InstructionFormat::A,
            DecodedInstruction::FType { .. } => InstructionFormat::F,
            DecodedInstruction::System { .. } => InstructionFormat::I,
            DecodedInstruction::Illegal => InstructionFormat::I, // TODO: Illegal instructions use I-type format I think
            DecodedInstruction::Compressed { .. } => InstructionFormat::C,
        }
    }

    /// Get the destination register (rd) if the instruction has one
    pub fn rd(&self) -> Option<u8> {
        match self {
            DecodedInstruction::RType { rd, .. }
            | DecodedInstruction::IType { rd, .. }
            | DecodedInstruction::UType { rd, .. }
            | DecodedInstruction::JType { rd, .. }
            | DecodedInstruction::AType { rd, .. }
            | DecodedInstruction::FType { rd, .. }
            | DecodedInstruction::System { rd, .. } => Some(*rd),
            DecodedInstruction::SType { .. }
            | DecodedInstruction::BType { .. }
            | DecodedInstruction::Illegal => None,
            DecodedInstruction::Compressed { expanded, .. } => expanded.rd(),
        }
    }

    /// Get the first source register (rs1) if the instruction has one
    pub fn rs1(&self) -> Option<u8> {
        match self {
            DecodedInstruction::RType { rs1, .. }
            | DecodedInstruction::IType { rs1, .. }
            | DecodedInstruction::SType { rs1, .. }
            | DecodedInstruction::BType { rs1, .. }
            | DecodedInstruction::AType { rs1, .. }
            | DecodedInstruction::FType { rs1, .. }
            | DecodedInstruction::System { rs1, .. } => Some(*rs1),
            DecodedInstruction::UType { .. }
            | DecodedInstruction::JType { .. }
            | DecodedInstruction::Illegal => None,
            DecodedInstruction::Compressed { expanded, .. } => expanded.rs1(),
        }
    }

    /// Get the second source register (rs2) if the instruction has one
    pub fn rs2(&self) -> Option<u8> {
        match self {
            DecodedInstruction::RType { rs2, .. }
            | DecodedInstruction::SType { rs2, .. }
            | DecodedInstruction::BType { rs2, .. }
            | DecodedInstruction::AType { rs2, .. } => Some(*rs2),
            DecodedInstruction::Compressed { expanded, .. } => expanded.rs2(),
            _ => None,
        }
    }

    /// Get the immediate value if the instruction has one
    pub fn imm(&self) -> Option<i32> {
        match self {
            DecodedInstruction::IType { imm, .. }
            | DecodedInstruction::SType { imm, .. }
            | DecodedInstruction::BType { imm, .. }
            | DecodedInstruction::UType { imm, .. }
            | DecodedInstruction::JType { imm, .. } => Some(*imm),
            DecodedInstruction::Compressed { expanded, .. } => expanded.imm(),
            _ => None,
        }
    }
}

impl fmt::Display for DecodedInstruction {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{} (format={:?}, opcode={:?})", self.mnemonic(), self.format(), self.opcode())
    }
}

/// Error types for instruction decoding
#[derive(Debug, Clone, PartialEq)]
pub enum DecodeError {
    /// Unknown or invalid opcode
    UnknownOpcode(u32),
    /// Invalid instruction format
    InvalidFormat,
    /// Reserved instruction encoding
    Reserved,
    /// Invalid function code combination
    InvalidFunct(u8, u8),
    /// Invalid program structure
    InvalidProgram(String),
}

impl fmt::Display for DecodeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            DecodeError::UnknownOpcode(opcode) => write!(f, "Unknown opcode: 0x{:02x}", opcode),
            DecodeError::InvalidFormat => write!(f, "Invalid instruction format"),
            DecodeError::Reserved => write!(f, "Reserved instruction"),
            DecodeError::InvalidFunct(funct3, funct7) => {
                write!(f, "Invalid function code: funct3=0x{:x}, funct7=0x{:x}", funct3, funct7)
            }
            DecodeError::InvalidProgram(msg) => write!(f, "Invalid program: {}", msg),
        }
    }
}

impl std::error::Error for DecodeError {}

pub type DecodeResult<T> = Result<T, DecodeError>;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_nop_creation() {
        let nop = DecodedInstruction::nop();

        assert!(nop.is_nop());
        assert_eq!(nop.mnemonic(), "addi");
        assert_eq!(nop.rd(), Some(0));
        assert_eq!(nop.rs1(), Some(0));
        assert_eq!(nop.imm(), Some(0));
        assert_eq!(nop.format(), InstructionFormat::I);
        assert_eq!(nop.opcode(), Opcode::OpImm);
        assert_eq!(nop.raw(), 0x00000013);
    }

    #[test]
    fn test_is_nop_detection() {
        let nop = DecodedInstruction::nop();
        assert!(nop.is_nop());

        // Regular addi instruction should not be NOP
        let regular_addi = DecodedInstruction::IType {
            raw: 0x02A00093,
            opcode: Opcode::OpImm,
            mnemonic: "addi".to_string(),
            rd: 1, // Different rd
            rs1: 0,
            imm: 42, // Different immediate
            funct3: 0,
            funct7: 0,
        };
        assert!(!regular_addi.is_nop());

        // R-type instruction should not be NOP
        let add_inst = DecodedInstruction::RType {
            raw: 0x002081B3,
            opcode: Opcode::Op,
            mnemonic: "add".to_string(),
            rd: 3,
            rs1: 1,
            rs2: 2,
            funct3: 0,
            funct7: 0,
        };
        assert!(!add_inst.is_nop());
    }

    #[test]
    fn test_illegal_creation() {
        let illegal = DecodedInstruction::illegal();

        assert!(illegal.check_for_illegal().is_some());
        assert!(!illegal.is_nop());
        assert_eq!(illegal.mnemonic(), "illegal");
        assert!(illegal.check_for_illegal().is_some());
        assert_eq!(illegal.rd(), None);
        assert_eq!(illegal.rs1(), None);
        assert_eq!(illegal.rs2(), None);
        assert_eq!(illegal.imm(), None);
        assert_eq!(illegal.format(), InstructionFormat::I);
    }

    #[test]
    fn test_illegal_detection() {
        let illegal = DecodedInstruction::illegal();
        assert!(illegal.check_for_illegal().is_some());

        let nop = DecodedInstruction::nop();
        assert!(!nop.check_for_illegal().is_some());

        let regular_inst = DecodedInstruction::IType {
            raw: 0x02A00093,
            opcode: Opcode::OpImm,
            mnemonic: "addi".to_string(),
            rd: 1,
            rs1: 0,
            imm: 42,
            funct3: 0,
            funct7: 0,
        };
        assert!(!regular_inst.check_for_illegal().is_some());
    }

    #[test]
    fn test_compressed_illegal_detection() {
        // Standard illegal
        let standard_illegal = DecodedInstruction::illegal();
        assert!(standard_illegal.check_for_illegal().is_some());

        // Compressed illegal (c.unimp)
        let compressed_illegal = DecodedInstruction::compressed_illegal();
        assert!(compressed_illegal.check_for_illegal().is_some()); // ✅ Should detect compressed illegal too
        assert!(compressed_illegal.is_compressed());

        // Regular compressed instruction (not illegal)
        let regular_compressed = DecodedInstruction::Compressed {
            raw: 0x0001,
            compressed_format: CompressedFormat::CI,
            compressed_mnemonic: "c.nop".to_string(),
            expanded: Box::new(DecodedInstruction::nop()),
        };
        assert!(!regular_compressed.check_for_illegal().is_some()); // Should NOT be illegal
        assert!(regular_compressed.is_compressed());
    }
    
    #[test]
    fn test_compressed_nop_detection() {
        // Standard NOP
        let standard_nop = DecodedInstruction::nop();
        assert!(standard_nop.is_nop());
        assert!(!standard_nop.is_compressed());
        
        // Compressed NOP (c.nop)
        let compressed_nop = DecodedInstruction::Compressed {
            raw: 0x0001,
            compressed_format: CompressedFormat::CI,
            compressed_mnemonic: "c.nop".to_string(),
            expanded: Box::new(DecodedInstruction::nop()),
        };
        assert!(compressed_nop.is_nop()); // ✅ Should detect compressed NOP too
        assert!(compressed_nop.is_compressed());
        assert!(!compressed_nop.check_for_illegal().is_some());
        
        // Regular compressed instruction (not a NOP)
        let compressed_addi = DecodedInstruction::Compressed {
            raw: 0x0421,
            compressed_format: CompressedFormat::CI,
            compressed_mnemonic: "c.addi".to_string(),
            expanded: Box::new(DecodedInstruction::IType {
                raw: 0x02A00093,
                opcode: Opcode::OpImm,
                mnemonic: "addi".to_string(),
                rd: 8,
                rs1: 8,
                imm: 5,
                funct3: 0,
                funct7: 0,
            }),
        };
        assert!(!compressed_addi.is_nop()); // Should NOT be NOP
        assert!(compressed_addi.is_compressed());
    }
    
    #[test]
    fn test_check_for_illegal() {
        // Legal instruction - should return None
        let legal = DecodedInstruction::nop();
        assert_eq!(legal.check_for_illegal(), None);
        assert!(!legal.check_for_illegal().is_some());
        
        // Standard illegal - should return Some(4) for 4 bytes
        let standard_illegal = DecodedInstruction::illegal();
        assert_eq!(standard_illegal.check_for_illegal(), Some(4));
        assert!(standard_illegal.check_for_illegal().is_some());
        
        // Compressed illegal - should return Some(2) for 2 bytes
        let compressed_illegal = DecodedInstruction::compressed_illegal();
        assert_eq!(compressed_illegal.check_for_illegal(), Some(2));
        assert!(compressed_illegal.check_for_illegal().is_some());
        
        // Regular compressed instruction - should return None
        let compressed_nop = DecodedInstruction::Compressed {
            raw: 0x0001,
            compressed_format: CompressedFormat::CI,
            compressed_mnemonic: "c.nop".to_string(),
            expanded: Box::new(DecodedInstruction::nop()),
        };
        assert_eq!(compressed_nop.check_for_illegal(), None);
        assert!(!compressed_nop.check_for_illegal().is_some());
    }
    
    #[test]
    fn test_check_for_nop() {
        // Legal non-NOP instruction - should return None
        let regular_inst = DecodedInstruction::IType {
            raw: 0x02A00093,
            opcode: Opcode::OpImm,
            mnemonic: "addi".to_string(),
            rd: 1,
            rs1: 0,
            imm: 42,
            funct3: 0,
            funct7: 0,
        };
        assert_eq!(regular_inst.check_for_nop(), None);
        
        // Standard NOP - should return Some(4) for 4 bytes
        let standard_nop = DecodedInstruction::nop();
        assert_eq!(standard_nop.check_for_nop(), Some(4));
        assert!(standard_nop.is_nop());
        
        // Compressed NOP - should return Some(2) for 2 bytes
        let compressed_nop = DecodedInstruction::Compressed {
            raw: 0x0001,
            compressed_format: CompressedFormat::CI,
            compressed_mnemonic: "c.nop".to_string(),
            expanded: Box::new(DecodedInstruction::nop()),
        };
        assert_eq!(compressed_nop.check_for_nop(), Some(2));
        assert!(compressed_nop.is_nop());
        
        // Regular compressed instruction - should return None
        let compressed_addi = DecodedInstruction::Compressed {
            raw: 0x0421,
            compressed_format: CompressedFormat::CI,
            compressed_mnemonic: "c.addi".to_string(),
            expanded: Box::new(DecodedInstruction::IType {
                raw: 0x02A00093,
                opcode: Opcode::OpImm,
                mnemonic: "addi".to_string(),
                rd: 8,
                rs1: 8,
                imm: 5,
                funct3: 0,
                funct7: 0,
            }),
        };
        assert_eq!(compressed_addi.check_for_nop(), None);
        assert!(!compressed_addi.is_nop());
    }
    
    #[test]
    fn test_compressed_illegal_constructor() {
        let compressed_illegal = DecodedInstruction::compressed_illegal();
        
        // Verify it's properly constructed
        assert!(compressed_illegal.is_compressed());
        assert_eq!(compressed_illegal.mnemonic(), "c.unimp");
        assert_eq!(compressed_illegal.check_for_illegal(), Some(2)); // 2 bytes for compressed
        assert!(compressed_illegal.check_for_illegal().is_some());
        assert_eq!(compressed_illegal.raw(), 0x0000);
        assert_eq!(compressed_illegal.length_bytes(), 2);
        
        // Verify the expanded form is correct
        if let Some(expanded) = compressed_illegal.expanded() {
            assert_eq!(expanded.mnemonic(), "illegal");
            assert_eq!(expanded.check_for_illegal(), Some(4)); // Expanded form would be 4 bytes
        } else {
            panic!("Expected compressed illegal to have expansion");
        }
    }
}
