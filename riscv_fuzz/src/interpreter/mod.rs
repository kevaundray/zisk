//! TODO

use crate::decoder::{InstructionDecoderRegistry, XLen};
use crate::instruction::{DecodeError, DecodeResult, DecodedInstruction};

/// Represents either a 16-bit compressed or 32-bit standard instruction
#[derive(Debug, Clone)]
enum RawInstruction {
    /// 16-bit compressed instruction
    Compressed(u16),
    /// 32-bit standard instruction  
    Standard(u32),
}

/// Modern RISC-V interpreter with type-safe decoding
pub struct RiscvDecoder {
    /// Unified decoder registry for all instruction types
    registry: InstructionDecoderRegistry,
}

impl RiscvDecoder {
    /// Create a new decoder with all standard decoders
    pub fn new() -> Self {
        Self {
            registry: InstructionDecoderRegistry::new(),
        }
    }

    /// Create a new decoder configured for the target XLEN (RV32 or RV64)
    pub fn new_with_xlen(xlen: XLen) -> Self {
        Self { registry: InstructionDecoderRegistry::with_xlen(xlen) }
    }

    /// Decode a buffer of 16-bit words into a vector of decoded instructions
    pub fn decode_program(&self, code: &[u16]) -> DecodeResult<Vec<DecodedInstruction>> {
        let mut instructions = Vec::new();
        let mut pc = 0;

        while pc < code.len() {
            let raw_inst = self.read_instruction(code, &mut pc)?;
            let decoded = match raw_inst {
                RawInstruction::Standard(inst) => self.decode_standard_instruction(inst)?,
                RawInstruction::Compressed(inst) => self.decode_compressed_instruction(inst)?,
            };
            instructions.push(decoded);
        }

        Ok(instructions)
    }

    /// Read the next instruction from the code buffer, handling both 16-bit and 32-bit instructions
    fn read_instruction(&self, code: &[u16], pc: &mut usize) -> DecodeResult<RawInstruction> {
        if *pc >= code.len() {
            return Err(DecodeError::InvalidProgram("Unexpected end of program".to_string()));
        }

        let first_half = code[*pc];
        *pc += 1;

        // Handle zero instruction (special case)
        // All instructions do not have first half == 0
        if first_half == 0 {
            return self.handle_zero_instruction(code, pc);
        }

        // Check if this is a 32-bit instruction (bits [1:0] == 11)
        if (first_half & 0x3) == 0x3 {
            // 32-bit instruction - read second half
            if *pc >= code.len() {
                return Err(DecodeError::InvalidProgram(
                    "Incomplete 32-bit instruction".to_string(),
                ));
            }

            let second_half = code[*pc];
            *pc += 1;

            let inst = (first_half as u32) | ((second_half as u32) << 16);
            Ok(RawInstruction::Standard(inst))
        } else {
            // 16-bit compressed instruction
            Ok(RawInstruction::Compressed(first_half))
        }
    }

    /// Handle zero instructions with proper compressed vs standard distinction
    ///
    /// Note: The program counter is pointing to the next instruction after the first zero that we saw
    fn handle_zero_instruction(
        &self,
        code: &[u16],
        pc: &mut usize,
    ) -> DecodeResult<RawInstruction> {
        if *pc == code.len() {
            // Last 16 bits are zero - this is a compressed illegal instruction
            return Ok(RawInstruction::Compressed(0x0000));
        }

        let next = code[*pc];
        if next == 0 {
            // Both halves are zero - this is a 32-bit illegal instruction
            *pc += 1; // Skip the instruction
            Ok(RawInstruction::Standard(0x00000000))
        } else {
            // First half is zero, second isn't
            //
            // This is the case where the first half is c.unimp and the
            // next instruction is another compressed or standard instruction.
            Ok(RawInstruction::Compressed(0x0000))
        }
    }

    /// Decode a 32-bit standard instruction using the unified registry
    fn decode_standard_instruction(&self, inst: u32) -> DecodeResult<DecodedInstruction> {
        // Registry handles all special cases internally
        self.registry.decode_standard(inst)
    }
    
    /// Decode a 16-bit compressed instruction using the unified registry
    fn decode_compressed_instruction(&self, inst: u16) -> DecodeResult<DecodedInstruction> {
        // Registry handles all special cases internally
        self.registry.decode_compressed(inst)
    }
}

impl Default for RiscvDecoder {
    fn default() -> Self {
        Self::new()
    }
}

/// Convenience function for decoding RISC-V instructions
pub fn decode_instructions(code: &[u16]) -> DecodeResult<Vec<DecodedInstruction>> {
    let decoder = RiscvDecoder::new();
    decoder.decode_program(code)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_decode_simple_program() {
        let decoder = RiscvDecoder::new();

        // Simple program: addi x1, x0, 42 (32-bit instruction)
        // Split into two 16-bit words: 0x0093, 0x02A0
        let code = [0x0093, 0x02A0];

        let result = decoder.decode_program(&code).unwrap();
        assert_eq!(result.len(), 1);

        let inst = &result[0];
        assert_eq!(inst.mnemonic(), "addi");
        assert_eq!(inst.rd(), Some(1));
        assert_eq!(inst.rs1(), Some(0));
        assert_eq!(inst.imm(), Some(42));
    }

    #[test]
    fn test_zero_instruction() {
        let decoder = RiscvDecoder::new();

        // Zero instruction: two zero 16-bit words → illegal instruction
        let code = [0x0000, 0x0000];

        let result = decoder.decode_program(&code).unwrap();
        assert_eq!(result.len(), 1);

        let inst = &result[0];
        assert_eq!(inst.mnemonic(), "illegal"); // All zeroes is now illegal
        assert!(inst.check_for_illegal().is_some());
        assert!(!inst.is_nop());
        assert_eq!(inst.rd(), None);
        assert_eq!(inst.rs1(), None);
        assert_eq!(inst.imm(), None);
    }

    #[test]
    fn test_convenience_function() {
        // Test the convenience function
        let code = [0x0093, 0x02A0]; // addi x1, x0, 42
        let result = decode_instructions(&code).unwrap();

        assert_eq!(result.len(), 1);
        assert_eq!(result[0].mnemonic(), "addi");
    }
}
