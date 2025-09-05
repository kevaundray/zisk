//! RISCV Instruction decoder
//!
//! We first go over some terminology to make it easier to parse this crate.
//!
//! An instruction is a single command that tells the CPU what to do.
//! RISCV instructions can be grouped in two different ways:
//!     - Extension: By functionality. This is the most common way to group instructions together. Example of extensions `IMAC` in `RV64IMAC`
//!     - Instruction formats: By how they are encoded. This is the lesser known way. Example of formats are I-type (Immediate operations and loads), R-type (Register-to-Register operations), S-type(Store operations)
//! This crate will for the most part, group instructions by instruction formats because we are implementing a Decoder.
//!
//! **Example**
//!
//! The exact format of a RISCV instruction may seem nebulous at first. Lets go over a quick example to illustrate:
//!
//! [0000000 | 00011 | 00010 | 000 | 00001 | 0110011]
//! [funct7  | rs2   | rs1   |funct3| rd   | opcode ]
//!
//! - The base opcode is `0110011` and this tells us that its an R-type instruction. ie it works with 2 source registers and a destination register.
//! - This by itself does not tell us whether it is an ADD, SUB, AND, OR, XOR
//! - To know the exact operation, we need to look at `funct3 and `funct7`
//! - `funct3` is `000` and that lets us know that it is either ADD, SUB or MUL
//! - `funct7` is `0000000` and that lets us know that it is `ADD`. If `funct7` was `0000001` then we would know its a MUL
//!
//! Above we manually decoded a raw bit string into its RISCV instruction. The Decoder will do this automatically.
//!
//! Note: One of the nice things about riscv is that no matter the instruction format, if it contains `funct3`, then it will be in the same position
//! regardless of the instruction format. This means the decoder is a lot simpler.

pub mod decoder;
pub mod instruction;
pub mod interpreter;

// Re-export the main types and functions
pub use decoder::*;
pub use instruction::*;
pub use interpreter::*;
