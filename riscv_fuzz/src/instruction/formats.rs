//! RISC-V instruction formats (types)
//!
//! RISC-V instructions can be grouped in two ways:
//! - **Extensions**: Group instructions by functionality (I, M, A, F, D, C)
//!   - `I` = Base integer instructions
//!   - `M` = Multiply/divide instructions  
//!   - `A` = Atomic instructions
//!   - `F/D` = Floating point instructions
//!   - `C` = Compressed (16-bit) instructions
//! - **Instruction Formats**: Group instructions by encoding layout (R, I, S, B, U, J)
//!
//! ⚠️  **Important**: Don't confuse the `I` extension with `I`-type format!
//!
//! ## Format Patterns and Usage
//!
//! ### R-Type (Register-Register)
//! 
//! Format layout:
//! - Bits 31-25: funct7 field
//! - Bits 24-20: rs2 (source register 2)
//! - Bits 19-15: rs1 (source register 1) 
//! - Bits 14-12: funct3 field
//! - Bits 11-7:  rd (destination register)
//! - Bits 6-0:   opcode
//! 
//! **Used for**: Arithmetic between two registers → third register
//! **Examples**: `add x1, x2, x3`, `sub x1, x2, x3`, `mul x1, x2, x3`
//! **Opcodes**: OP (0x33), OP-32 (0x3B)
//!
//! ### I-Type (Immediate)  
//! 
//! Format layout:
//! - Bits 31-20: imm[11:0] (immediate value)
//! - Bits 19-15: rs1 (source register 1)
//! - Bits 14-12: funct3 field  
//! - Bits 11-7:  rd (destination register)
//! - Bits 6-0:   opcode
//! 
//! **Used for**: Operations with 12-bit immediate values, loads
//! **Examples**: `addi x1, x2, 100`, `lw x1, 8(x2)`, `jalr x1, x2, 4`
//! **Opcodes**: LOAD (0x03), OP-IMM (0x13), OP-IMM-32 (0x1B), JALR (0x67)
//!
//! ### S-Type (Store)
//! 
//! Format layout:
//! - Bits 31-25: imm[11:5] (immediate upper bits)
//! - Bits 24-20: rs2 (source register 2)
//! - Bits 19-15: rs1 (source register 1)
//! - Bits 14-12: funct3 field
//! - Bits 11-7:  imm[4:0] (immediate lower bits)
//! - Bits 6-0:   opcode
//!   
//! **Used for**: Storing register values to memory
//! **Examples**: `sw x1, 8(x2)`, `sb x3, 0(x4)`
//! **Opcodes**: STORE (0x23)
//! **Note**: Immediate is split across two fields!
//!
//! ### B-Type (Branch)
//! 
//! Format layout:
//! - Bit 31:     imm[12]
//! - Bits 30-25: imm[10:5]
//! - Bits 24-20: rs2 (source register 2)
//! - Bits 19-15: rs1 (source register 1)
//! - Bits 14-12: funct3 field
//! - Bits 11-8:  imm[4:1]
//! - Bit 7:      imm[11] 
//! - Bits 6-0:   opcode
//! 
//! **Used for**: Conditional jumps (PC-relative)
//! **Examples**: `beq x1, x2, loop`, `bne x1, x0, end`  
//! **Opcodes**: BRANCH (0x63)
//! **Note**: Complex immediate encoding for ±4KB range
//!
//! ### U-Type (Upper Immediate)
//! 
//! Format layout:
//! - Bits 31-12: imm[31:12] (20-bit immediate)
//! - Bits 11-7:  rd (destination register)
//! - Bits 6-0:   opcode
//! 
//! **Used for**: Loading 20-bit constants into upper bits
//! **Examples**: `lui x1, 0x12345`, `auipc x1, 0x1000`
//! **Opcodes**: LUI (0x37), AUIPC (0x17)
//! **Note**: Immediate is left-shifted by 12 bits
//!
//! ### J-Type (Jump)
//! 
//! Format layout:
//! - Bit 31:     imm[20]
//! - Bits 30-21: imm[10:1]
//! - Bit 20:     imm[11]
//! - Bits 19-12: imm[19:12]
//! - Bits 11-7:  rd (destination register)
//! - Bits 6-0:   opcode
//! 
//! **Used for**: Unconditional jumps with large range  
//! **Examples**: `jal x1, function`, `jal x0, loop`
//! **Opcodes**: JAL (0x6F)
//! **Note**: Immediate encoding allows ±1MB jump range
//!
//! ## Format Selection Rules
//!
//! The RISC-V architects chose formats based on practical considerations:
//!
//! - **R-type**: Maximum flexibility for 3-operand instructions
//! - **I-type**: Most common format - immediate + 1 source + 1 dest  
//! - **S-type**: Optimized for `base + offset` memory stores
//! - **B-type**: Compact encoding for short conditional jumps
//! - **U-type**: Efficient for building large constants  
//! - **J-type**: Long-range jumps for function calls
//!
//! This design ensures efficient encoding while maintaining orthogonality.

use std::fmt;

/// RISC-V instruction format types
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum InstructionFormat {
    /// R-type: register-register operations (add, sub, etc.)
    /// Format: funct7 | rs2 | rs1 | funct3 | rd | opcode
    R,
    
    /// I-type: immediate operations and loads (addi, lw, etc.)
    /// Format: imm[11:0] | rs1 | funct3 | rd | opcode
    I,
    
    /// S-type: store operations (sw, sb, etc.)
    /// Format: imm[11:5] | rs2 | rs1 | funct3 | imm[4:0] | opcode
    S,
    
    /// B-type: conditional branches (beq, bne, etc.)
    /// Format: imm[12|10:5] | rs2 | rs1 | funct3 | imm[4:1|11] | opcode
    B,
    
    /// U-type: upper immediate operations (lui, auipc)
    /// Format: imm[31:12] | rd | opcode
    U,
    
    /// J-type: unconditional jumps (jal)
    /// Format: imm[20|10:1|11|19:12] | rd | opcode
    J,
    
    /// A-type: atomic operations (extension)
    /// Format: funct5 | aq | rl | rs2 | rs1 | funct3 | rd | opcode
    A,
    
    /// F-type: fence operations
    /// Format: fm | pred | succ | rs1 | funct3 | rd | opcode  
    F,

    /// C-type: compressed instructions (16-bit)  
    C,
}

/// RISC-V compressed instruction format types (16-bit)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum CompressedFormat {
    /// CR: Register format
    /// Layout: funct4 | rd/rs1 | rs2 | op
    CR,
    
    /// CI: Immediate format  
    /// Layout: funct3 | imm | rd/rs1 | imm | op
    CI,
    
    /// CSS: Stack-relative Store format
    /// Layout: funct3 | imm | rs2 | op
    CSS,
    
    /// CIW: Wide Immediate format
    /// Layout: funct3 | imm | rd' | op  
    CIW,
    
    /// CL: Load format
    /// Layout: funct3 | imm | rs1' | imm | rd' | op
    CL,
    
    /// CS: Store format
    /// Layout: funct3 | imm | rs1' | imm | rs2' | op
    CS,
    
    /// CA: Arithmetic format
    /// Layout: funct6 | rd'/rs1' | funct2 | rs2' | op
    CA,
    
    /// CB: Branch format
    /// Layout: funct3 | offset | rs1' | offset | op
    CB,
    
    /// CJ: Jump format
    /// Layout: funct3 | jump target | op
    CJ,
}

impl fmt::Display for InstructionFormat {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let name = match self {
            InstructionFormat::R => "R-type (register-register)",
            InstructionFormat::I => "I-type (immediate/load)",
            InstructionFormat::S => "S-type (store)",
            InstructionFormat::B => "B-type (branch)",
            InstructionFormat::U => "U-type (upper immediate)",
            InstructionFormat::J => "J-type (jump)",
            InstructionFormat::A => "A-type (atomic)",
            InstructionFormat::F => "F-type (fence)",
            InstructionFormat::C => "C-type (compressed)",
        };
        write!(f, "{}", name)
    }
}
