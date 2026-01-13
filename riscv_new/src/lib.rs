pub mod compressed_decoder;
pub mod standard_decoder;
pub mod target;

use crate::compressed_decoder::is_compressed;
use compressed_decoder::{decode_compressed_instruction, Instruction as CompressedInstruction};

pub use compressed_decoder::DecodeError as CompressedDecodeError;
pub use standard_decoder::{decode_standard_instruction, DecodeError, Instruction};
pub use target::Target;

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("Standard error: {0}")]
    Standard(DecodeError),
    #[error("Compressed error: {0}")]
    Compressed(CompressedDecodeError),
    #[error("Tried to read past end of file")]
    ReadingPastEOF,
}

/// Indicates whether an instruction was compressed or not
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum WasCompressed {
    /// 16-bit compressed instruction
    Yes,
    /// 32-bit standard instruction
    No,
}

/// High-level RISC-V instruction decoder with target configuration
pub struct InstructionDecoder {
    target: Target,
}

impl InstructionDecoder {
    /// Create a new decoder with default RV64GC target
    pub fn new() -> Self {
        Self { target: Target::rv64imac() }
    }

    /// Create a decoder with a specific target
    /// TODO: Remove, we will always assume imac
    pub fn with_target(target: Target) -> Self {
        Self { target }
    }

    /// Decode multiple instructions from a byte array (handles mixed 16/32-bit instructions)
    pub fn decode_bytes(&self, bytes: &[u8]) -> Result<Vec<(Instruction, WasCompressed)>, Error> {
        let expected_code_alignment = code_alignment(&self.target);
        assert!(
            bytes.len().is_multiple_of(expected_code_alignment),
            "code length = {} which is not a multiple of {}",
            bytes.len(),
            expected_code_alignment
        );

        let mut instructions = Vec::with_capacity(bytes.len() / 2);
        let mut i = 0;

        while i + 2 <= bytes.len() {
            // Read first 16-bit half
            let first_half = u16::from_le_bytes([bytes[i], bytes[i + 1]]);

            // Check if this is a 32-bit instruction
            if is_compressed(first_half) {
                // 16-bit compressed instruction
                let compressed_instruction = self.decode_compressed(first_half)?;
                // Convert from Compressed to Standard
                let instruction = Instruction::from(compressed_instruction);
                instructions.push((instruction, WasCompressed::Yes));
                i += 2;
            } else {
                // 32-bit instruction - need second half
                if i + 4 > bytes.len() {
                    return Err(Error::ReadingPastEOF);
                }

                let second_half = u16::from_le_bytes([bytes[i + 2], bytes[i + 3]]);
                let bits = (first_half as u32) | ((second_half as u32) << 16);

                let instruction = self.decode_standard(bits)?;
                instructions.push((instruction, WasCompressed::No));
                i += 4;
            }
        }

        Ok(instructions)
    }

    /// Decode a single 32-bit instruction
    fn decode_standard(&self, bits: u32) -> Result<Instruction, Error> {
        decode_standard_instruction(bits, &self.target).map_err(Error::Standard)
    }

    /// Decode a single 16-bit compressed instruction
    fn decode_compressed(&self, bits: u16) -> Result<CompressedInstruction, Error> {
        decode_compressed_instruction(bits, &self.target).map_err(Error::Compressed)
    }
}

impl Default for InstructionDecoder {
    fn default() -> Self {
        Self::new()
    }
}

/// Returns the code alignment in bytes
///
/// The code should either be a multiple of 2 and or 4.
/// It will be 2 if the compressed extension is enabled.
///
/// The cases where it will not be a multiple of 2 or 4, is if
/// the code was written with assembly and data was manually
/// added to the assembly file. One can force alignment in
/// assembly or in a linker script by adding .align 2
const fn code_alignment(target: &Target) -> usize {
    if target.compressed_enabled() {
        crate::compressed_decoder::Instruction::size()
    } else {
        crate::standard_decoder::Instruction::size()
    }
}
