//! RISC-V opcode definitions
//!
//! Contains the standard RISC-V opcodes as defined in the ISA specification.
//! Opcodes are the lower 7 bits of a 32-bit instruction that determine the
//! basic instruction category and format.
//!
//! ## RISC-V Opcode Encoding Patterns
//!
//! RISC-V opcodes follow systematic bit patterns that encode important information:
//!
//! ### Instruction Length (bits [1:0])
//! 
//! - `xx...xx11` → 32-bit or longer instruction (need to check bits [4:0] for longer formats)
//! - `xx...xx00` → 16-bit compressed instruction (quadrant 0)
//! - `xx...xx01` → 16-bit compressed instruction (quadrant 1)  
//! - `xx...xx10` → 16-bit compressed instruction (quadrant 2)
//!
//! For instructions ending in `11`, additional length encoding:
//! - `xxx...x011111` → 48-bit instruction (reserved)
//! - `xxx...x101111` → 64-bit instruction (reserved)  
//! - `xxx...x111111` → ≥80-bit instruction (reserved)
//!
//! Standard 32-bit instructions end in `11` but not `x1111` or `x11111`, making them easy to identify.
//! In practice, only 32-bit instructions are commonly used.
//!
//! ### Major Instruction Classes (bits [6:2])
//! Looking at bits [6:2] of 32-bit instructions (ignoring the `11` suffix):
//!
//! - `00000` (0x00) → LOAD     - Load from memory
//! - `00001` (0x04) → LOAD-FP  - Load floating point (F/D extensions)
//! - `00011` (0x0C) → MISC-MEM - Memory ordering (fence)
//! - `00100` (0x10) → OP-IMM   - Arithmetic with immediate  
//! - `00101` (0x14) → AUIPC    - Add upper immediate to PC
//! - `00110` (0x18) → OP-IMM-32- 32-bit immediate ops (RV64)
//! - `01000` (0x20) → STORE    - Store to memory
//! - `01001` (0x24) → STORE-FP - Store floating point
//! - `01011` (0x2C) → AMO      - Atomic memory operations
//! - `01100` (0x30) → OP       - Register-register ops
//! - `01101` (0x34) → LUI      - Load upper immediate
//! - `01110` (0x38) → OP-32    - 32-bit register ops (RV64)
//! - `11000` (0x60) → BRANCH   - Conditional branches
//! - `11001` (0x64) → JALR     - Jump and link register  
//! - `11011` (0x6C) → JAL      - Jump and link
//! - `11100` (0x70) → SYSTEM   - System instructions
//!
//! ### Pattern Examples
//! 
//! **Memory Operations** (loads/stores) cluster together:
//! - `LOAD  = 0b0000011` (bits 6:2 = `00000`)
//! - `STORE = 0b0100011` (bits 6:2 = `01000`) 
//!
//! **Arithmetic Operations** follow a pattern:
//! - `OP-IMM = 0b0010011` (bits 6:2 = `00100`) - immediate arithmetic
//! - `OP     = 0b0110011` (bits 6:2 = `01100`) - register arithmetic
//! 
//! Notice: OP-IMM + 0b0100000 = OP (immediate vs register pattern)
//!
//! **RV64 Extensions** add `1000` bit pattern:
//! - `OP-IMM    = 0b0010011` - 32-bit immediate ops
//! - `OP-IMM-32 = 0b0011011` - 64-bit word immediate ops (add 0b0001000)
//! - `OP        = 0b0110011` - 32-bit register ops  
//! - `OP-32     = 0b0111011` - 64-bit word register ops (add 0b0001000)
//!
//! **Control Flow** instructions are in the `11xxx` range:
//! - `BRANCH = 0b1100011` - conditional jumps
//! - `JALR   = 0b1100111` - indirect jumps  
//! - `JAL    = 0b1101111` - direct jumps
//! - `SYSTEM = 0b1110011` - system calls
//!
//! These patterns make it easy to categorize instructions just by looking at the opcode!

use std::fmt;
use super::{DecodeError, DecodeResult};

/// RISC-V opcodes (bits [6:0] of 32-bit instructions)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(u8)]
pub enum Opcode {
    /// Load instructions (lb, lh, lw, ld, lbu, lhu, lwu)
    Load = 0b0000011,
    
    /// Memory ordering instructions (fence, fence.i)  
    MiscMem = 0b0001111,
    
    /// Immediate arithmetic/logic operations (addi, slti, xori, etc.)
    OpImm = 0b0010011,
    
    /// Add upper immediate to PC (auipc)
    Auipc = 0b0010111,
    
    /// 32-bit immediate operations (addiw, slliw, etc.)
    OpImm32 = 0b0011011,
    
    /// Store instructions (sb, sh, sw, sd)
    Store = 0b0100011,
    
    /// Atomic memory operations (lr, sc, amo*)
    Amo = 0b0101111,
    
    /// Register-register operations (add, sub, mul, etc.)
    Op = 0b0110011,
    
    /// Load upper immediate (lui)
    Lui = 0b0110111,
    
    /// 32-bit register operations (addw, subw, etc.)
    Op32 = 0b0111011,
    
    /// Branch instructions (beq, bne, blt, etc.)
    Branch = 0b1100011,
    
    /// Jump and link register (jalr)
    Jalr = 0b1100111,
    
    /// Jump and link (jal)
    Jal = 0b1101111,
    
    /// System instructions (ecall, ebreak, csr)
    System = 0b1110011,

    /// Illegal/sentinel opcode for invalid/unsupported instructions
    /// Not produced by TryFrom; used only for DecodedInstruction::Illegal
    Illegal = 0x7F,
}

impl Opcode {
    /// Get the numeric value of the opcode
    pub fn value(self) -> u8 {
        self as u8
    }
    
    /// Get the numeric value as u32
    pub fn value_u32(self) -> u32 {
        self as u32
    }
    
    /// Get a human-readable description of the opcode
    pub fn description(self) -> &'static str {
        match self {
            Opcode::Load => "Load instructions (lb, lh, lw, ld, lbu, lhu, lwu)",
            Opcode::MiscMem => "Memory ordering instructions (fence, fence.i)",
            Opcode::OpImm => "Immediate arithmetic/logic operations (addi, slti, xori, etc.)",
            Opcode::Auipc => "Add upper immediate to PC (auipc)",
            Opcode::OpImm32 => "32-bit immediate operations (addiw, slliw, etc.)",
            Opcode::Store => "Store instructions (sb, sh, sw, sd)",
            Opcode::Amo => "Atomic memory operations (lr, sc, amo*)",
            Opcode::Op => "Register-register operations (add, sub, mul, etc.)",
            Opcode::Lui => "Load upper immediate (lui)",
            Opcode::Op32 => "32-bit register operations (addw, subw, etc.)",
            Opcode::Branch => "Branch instructions (beq, bne, blt, etc.)",
            Opcode::Jalr => "Jump and link register (jalr)",
            Opcode::Jal => "Jump and link (jal)",
            Opcode::System => "System instructions (ecall, ebreak, csr)",
            Opcode::Illegal => "Illegal/invalid instruction",
        }
    }
}

impl TryFrom<u8> for Opcode {
    type Error = DecodeError;
    
    fn try_from(value: u8) -> DecodeResult<Self> {
        match value {
            0b0000011 => Ok(Opcode::Load),
            0b0001111 => Ok(Opcode::MiscMem),
            0b0010011 => Ok(Opcode::OpImm),
            0b0010111 => Ok(Opcode::Auipc),
            0b0011011 => Ok(Opcode::OpImm32),
            0b0100011 => Ok(Opcode::Store),
            0b0101111 => Ok(Opcode::Amo),
            0b0110011 => Ok(Opcode::Op),
            0b0110111 => Ok(Opcode::Lui),
            0b0111011 => Ok(Opcode::Op32),
            0b1100011 => Ok(Opcode::Branch),
            0b1100111 => Ok(Opcode::Jalr),
            0b1101111 => Ok(Opcode::Jal),
            0b1110011 => Ok(Opcode::System),
            _ => Err(DecodeError::UnknownOpcode(value as u32)),
        }
    }
}

impl TryFrom<u32> for Opcode {
    type Error = DecodeError;
    
    fn try_from(value: u32) -> DecodeResult<Self> {
        if value > 0x7F {
            return Err(DecodeError::UnknownOpcode(value));
        }
        Self::try_from(value as u8)
    }
}

impl fmt::Display for Opcode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:?} (0x{:02x})", self, self.value())
    }
}
