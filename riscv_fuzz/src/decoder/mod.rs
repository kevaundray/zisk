//! RISC-V instruction decoders
//!
//! Unified decoding system for both standard (32-bit) and compressed (16-bit) RISC-V instructions.
//! Provides a clean, consistent interface for all instruction types while maintaining
//! type safety and performance.

pub mod compressed;
pub mod standard;
pub mod utils;

// Re-export individual decoders and utilities
pub use compressed::*;
pub use standard::*;
pub use utils::*;

// Main unified registry (replaces the separate registry.rs)
use crate::instruction::{
    DecodeError, DecodeResult, DecodedInstruction, InstructionFormat, Opcode,
};
use std::collections::HashMap;

/// Target XLEN for decoding semantics that depend on word size
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum XLen {
    X32,
    X64,
}

/// Trait for decoding 32-bit standard instructions of a specific format
pub trait StandardInstructionDecoder {
    /// The instruction format this decoder handles
    fn format(&self) -> InstructionFormat;

    /// Decode a 32-bit instruction into its constituent fields
    fn decode(&self, inst: u32) -> DecodeResult<DecodedInstruction>;

    /// Get the instruction mnemonic based on funct3/funct7 fields
    fn get_mnemonic(&self, funct3: u8, funct7: u8) -> DecodeResult<String>;
}

/// Trait for decoding 16-bit compressed instructions
pub trait CompressedInstructionDecoder {
    /// Which quadrant this decoder handles (0, 1, or 2)
    ///
    /// Note that what would be quadrant 3 is the bit pattern for uncompressed instructions
    ///
    /// Note: while uncompressed instructions are grouped into I/R/J type formats,
    /// compressed instructions are organized by quadrant (based on bits [1:0])
    fn quadrant(&self) -> u8;

    /// Decode a 16-bit compressed instruction
    fn decode(&self, inst: u16) -> DecodeResult<DecodedInstruction>;
}

/// Utility trait for field extraction from instruction words
pub trait FieldExtractor {
    /// Extract opcode (bits [6:0])
    fn opcode(&self) -> u8;

    /// Extract rd field (bits [11:7])
    fn rd(&self) -> u8;

    /// Extract rs1 field (bits [19:15])
    fn rs1(&self) -> u8;

    /// Extract rs2 field (bits [24:20])
    fn rs2(&self) -> u8;

    /// Extract funct3 field (bits [14:12])
    fn funct3(&self) -> u8;

    /// Extract funct7 field (bits [31:25])
    fn funct7(&self) -> u8;
}

impl FieldExtractor for u32 {
    fn opcode(&self) -> u8 {
        (*self & 0x7F) as u8
    }

    fn rd(&self) -> u8 {
        ((*self >> 7) & 0x1F) as u8
    }

    fn rs1(&self) -> u8 {
        ((*self >> 15) & 0x1F) as u8
    }

    fn rs2(&self) -> u8 {
        ((*self >> 20) & 0x1F) as u8
    }

    fn funct3(&self) -> u8 {
        ((*self >> 12) & 0x7) as u8
    }

    fn funct7(&self) -> u8 {
        ((*self >> 25) & 0x7F) as u8
    }
}

/// Unified instruction decoder registry for all RISC-V instruction types
pub struct InstructionDecoderRegistry {
    /// Registry for 32-bit standard instructions (by opcode)
    standard_decoders: HashMap<Opcode, Box<dyn StandardInstructionDecoder>>,
    /// Registry for 16-bit compressed instructions (by quadrant)
    compressed_decoders: HashMap<u8, Box<dyn CompressedInstructionDecoder>>,
    /// Target XLEN (affects shift-immediate validation and some compressed rules)
    xlen: XLen,
}

impl InstructionDecoderRegistry {
    /// Create a new unified registry with all standard RISC-V decoders
    pub fn new() -> Self {
        Self::with_xlen(XLen::X64)
    }

    /// Create a new registry for a specific XLEN (RV32 or RV64)
    pub fn with_xlen(xlen: XLen) -> Self {
        let mut registry = Self {
            standard_decoders: HashMap::new(),
            compressed_decoders: HashMap::new(),
            xlen,
        };

        registry.register_standard_decoders();
        registry.register_compressed_decoders();
        registry
    }

    /// Register all standard RISC-V instruction decoders
    fn register_standard_decoders(&mut self) {
        // I-type decoders
        self.register_standard(Opcode::Load, Box::new(ITypeDecoder::new(self.xlen)));
        self.register_standard(Opcode::OpImm, Box::new(ITypeDecoder::new(self.xlen)));
        self.register_standard(Opcode::OpImm32, Box::new(ITypeDecoder::new(self.xlen)));
        self.register_standard(Opcode::Jalr, Box::new(ITypeDecoder::new(self.xlen)));

        // R-type decoders
        self.register_standard(Opcode::Op, Box::new(RTypeDecoder::new()));
        self.register_standard(Opcode::Op32, Box::new(RTypeDecoder::new_rv64_word()));

        // S-type decoders
        self.register_standard(Opcode::Store, Box::new(STypeDecoder::new()));

        // B-type decoders
        self.register_standard(Opcode::Branch, Box::new(BTypeDecoder::new()));

        // U-type decoders
        self.register_standard(Opcode::Lui, Box::new(UTypeDecoder));
        self.register_standard(Opcode::Auipc, Box::new(UTypeDecoder));

        // J-type decoders
        self.register_standard(Opcode::Jal, Box::new(JTypeDecoder));

        // Fence operations
        self.register_standard(Opcode::MiscMem, Box::new(FenceDecoder));

        // System instructions
        self.register_standard(Opcode::System, Box::new(SystemDecoder));

        // Atomic operations (RV32A/RV64A)
        self.register_standard(Opcode::Amo, Box::new(ATypeDecoder::new()));
    }

    /// Register all compressed RISC-V instruction decoders
    fn register_compressed_decoders(&mut self) {
        // Register quadrant-based decoders
        self.register_compressed(0, Box::new(Quadrant0Decoder));
        self.register_compressed(1, Box::new(Quadrant1Decoder::new(self.xlen)));

        // Quadrant 2 (complete implementation)
        self.register_compressed(2, Box::new(Quadrant2Decoder::new(self.xlen)));
    }

    /// Register a decoder for a specific standard instruction opcode
    pub fn register_standard(
        &mut self,
        opcode: Opcode,
        decoder: Box<dyn StandardInstructionDecoder>,
    ) {
        self.standard_decoders.insert(opcode, decoder);
    }

    /// Register a decoder for a specific compressed instruction quadrant  
    pub fn register_compressed(
        &mut self,
        quadrant: u8,
        decoder: Box<dyn CompressedInstructionDecoder>,
    ) {
        self.compressed_decoders.insert(quadrant, decoder);
    }

    /// Decode a 32-bit standard instruction
    pub fn decode_standard(&self, inst: u32) -> DecodeResult<DecodedInstruction> {
        // Handle special cases first
        if inst == 0 {
            // All zeroes is treated as an illegal instruction
            return Ok(DecodedInstruction::illegal());
        }

        let opcode = Opcode::try_from(inst & 0x7F)?;
        let decoder =
            self.standard_decoders.get(&opcode).ok_or(DecodeError::UnknownOpcode(opcode as u32))?;

        decoder.decode(inst)
    }

    /// Decode a 16-bit compressed instruction
    pub fn decode_compressed(&self, inst: u16) -> DecodeResult<DecodedInstruction> {
        // Handle special case for all-zero compressed instruction
        if inst == 0x0000 {
            return Ok(DecodedInstruction::compressed_illegal());
        }

        let quadrant = (inst & 0x3) as u8;
        let decoder =
            self.compressed_decoders.get(&quadrant).ok_or(DecodeError::InvalidProgram(format!(
                "No decoder registered for compressed quadrant {}",
                quadrant
            )))?;

        decoder.decode(inst)
    }

    /// Check if a decoder is registered for the given standard opcode
    pub fn has_standard_decoder(&self, opcode: Opcode) -> bool {
        self.standard_decoders.contains_key(&opcode)
    }

    /// Check if a decoder is registered for the given compressed quadrant
    pub fn has_compressed_decoder(&self, quadrant: u8) -> bool {
        self.compressed_decoders.contains_key(&quadrant)
    }

    /// Get the list of supported standard opcodes
    pub fn supported_opcodes(&self) -> Vec<Opcode> {
        self.standard_decoders.keys().copied().collect()
    }

    /// Get the list of supported compressed quadrants
    pub fn supported_quadrants(&self) -> Vec<u8> {
        self.compressed_decoders.keys().copied().collect()
    }
}

impl Default for InstructionDecoderRegistry {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::instruction::{InstructionFormat, Opcode};

    #[test]
    fn test_unified_registry_creation() {
        let registry = InstructionDecoderRegistry::new();

        // Check that standard decoders are registered
        assert!(registry.has_standard_decoder(Opcode::Load));
        assert!(registry.has_standard_decoder(Opcode::Op));
        assert!(registry.has_standard_decoder(Opcode::Branch));
        assert!(registry.has_standard_decoder(Opcode::Jal));
        assert!(registry.has_standard_decoder(Opcode::System));

        // Check that compressed decoders are registered
        assert!(registry.has_compressed_decoder(0)); // Quadrant 0
        assert!(registry.has_compressed_decoder(1)); // Quadrant 1
        assert!(registry.has_compressed_decoder(2)); // Quadrant 2

        // Check that we have a reasonable number of supported opcodes and quadrants
        let supported_opcodes = registry.supported_opcodes();
        assert!(supported_opcodes.len() >= 10, "Should support at least 10 opcodes");

        let supported_quadrants = registry.supported_quadrants();
        assert!(supported_quadrants.len() >= 3, "Should support at least 3 quadrants");
        assert!(supported_quadrants.contains(&0));
        assert!(supported_quadrants.contains(&1));
        assert!(supported_quadrants.contains(&2));
    }

    #[test]
    fn test_standard_instruction_decoding() {
        let registry = InstructionDecoderRegistry::new();

        // Test add x1, x2, x3
        let inst = 0x003100B3u32;
        let decoded = registry.decode_standard(inst).unwrap();

        assert_eq!(decoded.format(), InstructionFormat::R);
        assert_eq!(decoded.opcode(), Opcode::Op);
        assert_eq!(decoded.mnemonic(), "add");
        assert_eq!(decoded.rd(), Some(1));
        assert_eq!(decoded.rs1(), Some(2));
        assert_eq!(decoded.rs2(), Some(3));
    }

    #[test]
    fn test_compressed_instruction_decoding() {
        let registry = InstructionDecoderRegistry::new();

        // Test c.nop (0x0001)
        let inst = 0x0001u16;
        let decoded = registry.decode_compressed(inst).unwrap();

        assert!(decoded.is_compressed());
        assert_eq!(decoded.mnemonic(), "c.nop");
        assert_eq!(decoded.format(), InstructionFormat::C);

        // Check that it expands to a proper nop
        if let Some(expanded) = decoded.expanded() {
            assert!(expanded.is_nop());
            assert_eq!(expanded.mnemonic(), "addi");
        } else {
            panic!("Expected compressed instruction to have expansion");
        }
    }

    #[test]
    fn test_zero_instruction_handling() {
        let registry = InstructionDecoderRegistry::new();

        // Standard zero instruction should be illegal
        let decoded_standard = registry.decode_standard(0x00000000).unwrap();
        assert!(decoded_standard.check_for_illegal().is_some());

        // Compressed zero instruction should be c.unimp
        let decoded_compressed = registry.decode_compressed(0x0000).unwrap();
        assert!(decoded_compressed.is_compressed());
        assert_eq!(decoded_compressed.mnemonic(), "c.unimp");
    }

    #[test]
    fn test_rv32_slli_shamt_reserved() {
        // Build slli x1, x1, 32 (shamt[5]=1) → reserved on RV32
        // imm[5]=1 → bit 25 set, funct3=001, opcode=0x13
        let inst: u32 = 0x02009093; // computed composition

        let registry = InstructionDecoderRegistry::with_xlen(XLen::X32);
        let res = registry.decode_standard(inst);
        assert!(res.is_err());
    }

    #[test]
    fn test_rv64_slli_shamt_32_ok() {
        // Same instruction should be valid on RV64
        let inst: u32 = 0x02009093;
        let registry = InstructionDecoderRegistry::with_xlen(XLen::X64);
        let res = registry.decode_standard(inst).expect("RV64 slli decode failed");
        assert_eq!(res.mnemonic(), "slli");
    }

    #[test]
    fn test_rv32_c_slli_shamt_reserved() {
        // c.slli with shamt[5]=1 is reserved on RV32
        // Quadrant2, funct3=000, bit12=1, rd=1, shamt[4:0]=0, low=10
        let inst: u16 = 0x1082;
        let registry = InstructionDecoderRegistry::with_xlen(XLen::X32);
        let res = registry.decode_compressed(inst);
        assert!(res.is_err());
    }

    #[test]
    fn test_rv32_c_srli_srai_shamt_reserved() {
        let registry = InstructionDecoderRegistry::with_xlen(XLen::X32);
        // c.srli: Quadrant1, funct3=100, bit12=1, sub=00, rd'=x8, shamt[4:0]=0, low=01
        let srli: u16 = 0x9001;
        let res1 = registry.decode_compressed(srli);
        assert!(res1.is_err());

        // c.srai: sub=01
        let srai: u16 = 0x9401;
        let res2 = registry.decode_compressed(srai);
        assert!(res2.is_err());
    }

    #[test]
    fn test_rv64_sraiw_decoding() {
        // sraiw x1, x1, 1 → OP-IMM-32, funct3=101, funct7=0100000
        let inst: u32 = 0x4010D09B;
        let registry = InstructionDecoderRegistry::with_xlen(XLen::X64);
        let decoded = registry.decode_standard(inst).expect("sraiw decode failed");
        assert_eq!(decoded.opcode(), Opcode::OpImm32);
        assert_eq!(decoded.mnemonic(), "sraiw");
    }
}
