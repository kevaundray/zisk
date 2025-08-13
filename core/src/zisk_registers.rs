//! Zisk registers
//!
//! # RISC-V registers memory mapping
//!
//! The 32 8-bytes RISC-V registers are mapped to RW memory starting at address SYS_ADDR.
//! They occupy 32x8=256 bytes of memory space.
//!
//! Ref: https://riscv-non-isa.github.io/riscv-elf-psabi-doc/#_register_convention
//!
//! | ABI name | X name  | Usage                                     |
//! |----------|---------|-------------------------------------------|
//! | REG_ZERO | REG_X0  | Read always as zero                       |
//! | REG_RA   | REG_X1  | Return address                            |
//! | REG_SP   | REG_X2  | Stack pointer                             |
//! | REG_GP   | REG_X3  | Global pointer                            |
//! | REG_TP   | REG_X4  | Thread pointer                            |
//! | REG_T0   | REG_X5  | Temporary register 0                      |
//! | REG_T1   | REG_X6  | Temporary register 1                      |
//! | REG_T2   | REG_X7  | Temporary register 2                      |
//! | REG_S0   | REG_X8  | Saved register 0 / frame pointer          |
//! | REG_S1   | REG_X9  | Saved register 1                          |
//! | REG_A0   | REG_X10 | Function argument 0 / return value 0      |
//! | REG_A1   | REG_X11 | Function argument 1 / return value 1      |
//! | REG_A2   | REG_X12 | Function argument 2                       |
//! | REG_A3   | REG_X13 | Function argument 3                       |
//! | REG_A4   | REG_X14 | Function argument 4                       |
//! | REG_A5   | REG_X15 | Function argument 5                       |
//! | REG_A6   | REG_X16 | Function argument 6                       |
//! | REG_A7   | REG_X17 | Function argument 7                       |
//! | REG_S2   | REG_X18 | Saved register 2                          |
//! | REG_S3   | REG_X19 | Saved register 3                          |
//! | REG_S4   | REG_X20 | Saved register 4                          |
//! | REG_S5   | REG_X21 | Saved register 5                          |
//! | REG_S6   | REG_X22 | Saved register 6                          |
//! | REG_S7   | REG_X23 | Saved register 7                          |
//! | REG_S8   | REG_X24 | Saved register 8                          |
//! | REG_S9   | REG_X25 | Saved register 9                          |
//! | REG_S10  | REG_X26 | Saved register 10                         |
//! | REG_S11  | REG_X27 | Saved register 11                         |
//! | REG_T3   | REG_X28 | Temporary register 3                      |
//! | REG_T4   | REG_X29 | Temporary register 4                      |
//! | REG_T5   | REG_X30 | Temporary register 5                      |
//! | REG_T6   | REG_X31 | Temporary register 6                      |
//!
//! # RISC-V Floating Point registers memory mapping (RISC-V psABI)
//!
//! The 32 8-bytes RISC-V floating point registers are mapped to RW memory starting
//! at address FREG_FIRST (after integer registers). They occupy 32x8=256 bytes.
//!
//! | ABI name | F name   | Usage                                     |
//! |----------|----------|-------------------------------------------|
//! | FREG_FT0 | FREG_F0  | Floating point temporary 0 (caller-saved)|
//! | FREG_FT1 | FREG_F1  | Floating point temporary 1 (caller-saved)|
//! | FREG_FT2 | FREG_F2  | Floating point temporary 2 (caller-saved)|
//! | FREG_FT3 | FREG_F3  | Floating point temporary 3 (caller-saved)|
//! | FREG_FT4 | FREG_F4  | Floating point temporary 4 (caller-saved)|
//! | FREG_FT5 | FREG_F5  | Floating point temporary 5 (caller-saved)|
//! | FREG_FT6 | FREG_F6  | Floating point temporary 6 (caller-saved)|
//! | FREG_FT7 | FREG_F7  | Floating point temporary 7 (caller-saved)|
//! | FREG_FS0 | FREG_F8  | Floating point saved 0 (callee-saved)    |
//! | FREG_FS1 | FREG_F9  | Floating point saved 1 (callee-saved)    |
//! | FREG_FA0 | FREG_F10 | FP argument/return 0                      |
//! | FREG_FA1 | FREG_F11 | FP argument/return 1                      |
//! | FREG_FA2 | FREG_F12 | FP argument 2                             |
//! | FREG_FA3 | FREG_F13 | FP argument 3                             |
//! | FREG_FA4 | FREG_F14 | FP argument 4                             |
//! | FREG_FA5 | FREG_F15 | FP argument 5                             |
//! | FREG_FA6 | FREG_F16 | FP argument 6                             |
//! | FREG_FA7 | FREG_F17 | FP argument 7                             |
//! | FREG_FS2 | FREG_F18 | Floating point saved 2 (callee-saved)    |
//! | FREG_FS3 | FREG_F19 | Floating point saved 3 (callee-saved)    |
//! | FREG_FS4 | FREG_F20 | Floating point saved 4 (callee-saved)    |
//! | FREG_FS5 | FREG_F21 | Floating point saved 5 (callee-saved)    |
//! | FREG_FS6 | FREG_F22 | Floating point saved 6 (callee-saved)    |
//! | FREG_FS7 | FREG_F23 | Floating point saved 7 (callee-saved)    |
//! | FREG_FS8 | FREG_F24 | Floating point saved 8 (callee-saved)    |
//! | FREG_FS9 | FREG_F25 | Floating point saved 9 (callee-saved)    |
//! | FREG_FS10| FREG_F26 | Floating point saved 10 (callee-saved)   |
//! | FREG_FS11| FREG_F27 | Floating point saved 11 (callee-saved)   |
//! | FREG_FT8 | FREG_F28 | Floating point temporary 8 (caller-saved)|
//! | FREG_FT9 | FREG_F29 | Floating point temporary 9 (caller-saved)|
//! | FREG_FT10| FREG_F30 | Floating point temporary 10(caller-saved)|
//! | FREG_FT11| FREG_F31 | Floating point temporary 11(caller-saved)|
//!
//! # RISC-V Floating Point Control and Status Registers
//!
//! The floating point extension requires several CSRs for proper operation:
//!
//! | CSR Name | Address | Description                              |
//! |----------|---------|------------------------------------------|
//! | fcsr     | 0x003   | Floating-point control and status reg   |
//! | frm      | 0x002   | Floating-point rounding mode            |
//! | fflags   | 0x001   | Floating-point exception flags          |
//!
//! The FCSR register contains:
//! - Bits 7:5 - frm (rounding mode): 000=RNE, 001=RTZ, 010=RDN, 011=RUP, 100=RMM
//! - Bits 4:0 - fflags (exception flags): NV, DZ, OF, UF, NX

use crate::SYS_ADDR;

// Registers memory address definitions
pub const REG_FIRST: u64 = SYS_ADDR;

// These are the generic register names, i.e. REG_Xn.
pub const REG_X0: u64 = REG_FIRST;
pub const REG_X1: u64 = REG_FIRST + 8;
pub const REG_X2: u64 = REG_FIRST + 2_u64 * 8;
pub const REG_X3: u64 = REG_FIRST + 3_u64 * 8;
pub const REG_X4: u64 = REG_FIRST + 4_u64 * 8;
pub const REG_X5: u64 = REG_FIRST + 5_u64 * 8;
pub const REG_X6: u64 = REG_FIRST + 6_u64 * 8;
pub const REG_X7: u64 = REG_FIRST + 7_u64 * 8;
pub const REG_X8: u64 = REG_FIRST + 8_u64 * 8;
pub const REG_X9: u64 = REG_FIRST + 9_u64 * 8;
pub const REG_X10: u64 = REG_FIRST + 10_u64 * 8;
pub const REG_X11: u64 = REG_FIRST + 11_u64 * 8;
pub const REG_X12: u64 = REG_FIRST + 12_u64 * 8;
pub const REG_X13: u64 = REG_FIRST + 13_u64 * 8;
pub const REG_X14: u64 = REG_FIRST + 14_u64 * 8;
pub const REG_X15: u64 = REG_FIRST + 15_u64 * 8;
pub const REG_X16: u64 = REG_FIRST + 16_u64 * 8;
pub const REG_X17: u64 = REG_FIRST + 17_u64 * 8;
pub const REG_X18: u64 = REG_FIRST + 18_u64 * 8;
pub const REG_X19: u64 = REG_FIRST + 19_u64 * 8;
pub const REG_X20: u64 = REG_FIRST + 20_u64 * 8;
pub const REG_X21: u64 = REG_FIRST + 21_u64 * 8;
pub const REG_X22: u64 = REG_FIRST + 22_u64 * 8;
pub const REG_X23: u64 = REG_FIRST + 23_u64 * 8;
pub const REG_X24: u64 = REG_FIRST + 24_u64 * 8;
pub const REG_X25: u64 = REG_FIRST + 25_u64 * 8;
pub const REG_X26: u64 = REG_FIRST + 26_u64 * 8;
pub const REG_X27: u64 = REG_FIRST + 27_u64 * 8;
pub const REG_X28: u64 = REG_FIRST + 28_u64 * 8;
pub const REG_X29: u64 = REG_FIRST + 29_u64 * 8;
pub const REG_X30: u64 = REG_FIRST + 30_u64 * 8;
pub const REG_X31: u64 = REG_FIRST + 31_u64 * 8;

pub const REG_LAST: u64 = REG_X31;

// Floating point registers memory address definitions
pub const FREG_FIRST: u64 = REG_LAST + 8;

// These are the generic floating point register names, i.e. FREG_Fn.
pub const FREG_F0: u64 = FREG_FIRST;
pub const FREG_F1: u64 = FREG_FIRST + 8;
pub const FREG_F2: u64 = FREG_FIRST + 2_u64 * 8;
pub const FREG_F3: u64 = FREG_FIRST + 3_u64 * 8;
pub const FREG_F4: u64 = FREG_FIRST + 4_u64 * 8;
pub const FREG_F5: u64 = FREG_FIRST + 5_u64 * 8;
pub const FREG_F6: u64 = FREG_FIRST + 6_u64 * 8;
pub const FREG_F7: u64 = FREG_FIRST + 7_u64 * 8;
pub const FREG_F8: u64 = FREG_FIRST + 8_u64 * 8;
pub const FREG_F9: u64 = FREG_FIRST + 9_u64 * 8;
pub const FREG_F10: u64 = FREG_FIRST + 10_u64 * 8;
pub const FREG_F11: u64 = FREG_FIRST + 11_u64 * 8;
pub const FREG_F12: u64 = FREG_FIRST + 12_u64 * 8;
pub const FREG_F13: u64 = FREG_FIRST + 13_u64 * 8;
pub const FREG_F14: u64 = FREG_FIRST + 14_u64 * 8;
pub const FREG_F15: u64 = FREG_FIRST + 15_u64 * 8;
pub const FREG_F16: u64 = FREG_FIRST + 16_u64 * 8;
pub const FREG_F17: u64 = FREG_FIRST + 17_u64 * 8;
pub const FREG_F18: u64 = FREG_FIRST + 18_u64 * 8;
pub const FREG_F19: u64 = FREG_FIRST + 19_u64 * 8;
pub const FREG_F20: u64 = FREG_FIRST + 20_u64 * 8;
pub const FREG_F21: u64 = FREG_FIRST + 21_u64 * 8;
pub const FREG_F22: u64 = FREG_FIRST + 22_u64 * 8;
pub const FREG_F23: u64 = FREG_FIRST + 23_u64 * 8;
pub const FREG_F24: u64 = FREG_FIRST + 24_u64 * 8;
pub const FREG_F25: u64 = FREG_FIRST + 25_u64 * 8;
pub const FREG_F26: u64 = FREG_FIRST + 26_u64 * 8;
pub const FREG_F27: u64 = FREG_FIRST + 27_u64 * 8;
pub const FREG_F28: u64 = FREG_FIRST + 28_u64 * 8;
pub const FREG_F29: u64 = FREG_FIRST + 29_u64 * 8;
pub const FREG_F30: u64 = FREG_FIRST + 30_u64 * 8;
pub const FREG_F31: u64 = FREG_FIRST + 31_u64 * 8;

pub const FREG_LAST: u64 = FREG_F31;

// RISC-V floating point ABI register names (psABI specification)
// ft0–ft7 → f0–f7 (caller-saved temps)
pub const FREG_FT0: u64 = FREG_F0; // ft0  -> f0   (caller-saved temp)
pub const FREG_FT1: u64 = FREG_F1; // ft1  -> f1   (caller-saved temp)
pub const FREG_FT2: u64 = FREG_F2; // ft2  -> f2   (caller-saved temp)
pub const FREG_FT3: u64 = FREG_F3; // ft3  -> f3   (caller-saved temp)
pub const FREG_FT4: u64 = FREG_F4; // ft4  -> f4   (caller-saved temp)
pub const FREG_FT5: u64 = FREG_F5; // ft5  -> f5   (caller-saved temp)
pub const FREG_FT6: u64 = FREG_F6; // ft6  -> f6   (caller-saved temp)
pub const FREG_FT7: u64 = FREG_F7; // ft7  -> f7   (caller-saved temp)

// fs0–fs1 → f8–f9 (callee-saved)
pub const FREG_FS0: u64 = FREG_F8; // fs0  -> f8   (callee-saved)
pub const FREG_FS1: u64 = FREG_F9; // fs1  -> f9   (callee-saved)

// fa0–fa7 → f10–f17 (args/returns)
pub const FREG_FA0: u64 = FREG_F10; // fa0  -> f10  (argument/return)
pub const FREG_FA1: u64 = FREG_F11; // fa1  -> f11  (argument/return)
pub const FREG_FA2: u64 = FREG_F12; // fa2  -> f12  (argument)
pub const FREG_FA3: u64 = FREG_F13; // fa3  -> f13  (argument)
pub const FREG_FA4: u64 = FREG_F14; // fa4  -> f14  (argument)
pub const FREG_FA5: u64 = FREG_F15; // fa5  -> f15  (argument)
pub const FREG_FA6: u64 = FREG_F16; // fa6  -> f16  (argument)
pub const FREG_FA7: u64 = FREG_F17; // fa7  -> f17  (argument)

// fs2–fs11 → f18–f27 (callee-saved)
pub const FREG_FS2: u64 = FREG_F18; // fs2  -> f18  (callee-saved)
pub const FREG_FS3: u64 = FREG_F19; // fs3  -> f19  (callee-saved)
pub const FREG_FS4: u64 = FREG_F20; // fs4  -> f20  (callee-saved)
pub const FREG_FS5: u64 = FREG_F21; // fs5  -> f21  (callee-saved)
pub const FREG_FS6: u64 = FREG_F22; // fs6  -> f22  (callee-saved)
pub const FREG_FS7: u64 = FREG_F23; // fs7  -> f23  (callee-saved)
pub const FREG_FS8: u64 = FREG_F24; // fs8  -> f24  (callee-saved)
pub const FREG_FS9: u64 = FREG_F25; // fs9  -> f25  (callee-saved)
pub const FREG_FS10: u64 = FREG_F26; // fs10 -> f26  (callee-saved)
pub const FREG_FS11: u64 = FREG_F27; // fs11 -> f27  (callee-saved)

// ft8–ft11 → f28–f31 (caller-saved temps)
pub const FREG_FT8: u64 = FREG_F28; // ft8  -> f28  (caller-saved temp)
pub const FREG_FT9: u64 = FREG_F29; // ft9  -> f29  (caller-saved temp)
pub const FREG_FT10: u64 = FREG_F30; // ft10 -> f30  (caller-saved temp)
pub const FREG_FT11: u64 = FREG_F31; // ft11 -> f31  (caller-saved temp)

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Mem;

    #[test]
    fn test_floating_point_registers() {
        // Test memory layout
        assert_eq!(FREG_FIRST, REG_LAST + 8);
        assert_eq!(FREG_F0, FREG_FIRST);
        assert_eq!(FREG_F1, FREG_FIRST + 8);
        assert_eq!(FREG_F31, FREG_FIRST + 31 * 8);
        assert_eq!(FREG_LAST, FREG_F31);

        // Test address calculations
        for i in 0..32 {
            let expected_addr = FREG_FIRST + (i * 8);
            assert_eq!(expected_addr, FREG_FIRST + (i * 8));
        }

        // Test register detection functions
        assert!(Mem::address_is_freg(FREG_F0));
        assert!(Mem::address_is_freg(FREG_F1));
        assert!(Mem::address_is_freg(FREG_F31));
        assert!(!Mem::address_is_freg(0x1000)); // Random address
        assert!(!Mem::address_is_freg(REG_X0)); // Integer register

        // Test register index conversion
        assert_eq!(Mem::address_to_freg_index(FREG_F0), 0);
        assert_eq!(Mem::address_to_freg_index(FREG_F1), 1);
        assert_eq!(Mem::address_to_freg_index(FREG_F31), 31);

        // Test ABI names match psABI specification
        assert_eq!(FREG_FT0, FREG_F0); // ft0 -> f0
        assert_eq!(FREG_FS0, FREG_F8); // fs0 -> f8
        assert_eq!(FREG_FA0, FREG_F10); // fa0 -> f10
        assert_eq!(FREG_FT8, FREG_F28); // ft8 -> f28
    }
}

// ABI register names.
pub const REG_ZERO: u64 = REG_X0;
pub const REG_RA: u64 = REG_X1; // Return address
pub const REG_SP: u64 = REG_X2; // Stack pointer
pub const REG_GP: u64 = REG_X3; // Global pointer
pub const REG_TP: u64 = REG_X4; // Thread pointer
pub const REG_T0: u64 = REG_X5; // Temporary register 0
pub const REG_T1: u64 = REG_X6; // Temporary register 1
pub const REG_T2: u64 = REG_X7; // Temporary register 2
pub const REG_S0: u64 = REG_X8; // Saved register 0 / frame pointer
pub const REG_S1: u64 = REG_X9; // Saved register 1
pub const REG_A0: u64 = REG_X10; // Function argument 0 / return value 0
pub const REG_A1: u64 = REG_X11; // Function argument 1 / return value 1
pub const REG_A2: u64 = REG_X12; // Function argument 2
pub const REG_A3: u64 = REG_X13; // Function argument 3
pub const REG_A4: u64 = REG_X14; // Function argument 4
pub const REG_A5: u64 = REG_X15; // Function argument 5
pub const REG_A6: u64 = REG_X16; // Function argument 6
pub const REG_A7: u64 = REG_X17; // Function argument 7
pub const REG_S2: u64 = REG_X18; // Saved register 2
pub const REG_S3: u64 = REG_X19; // Saved register 3
pub const REG_S4: u64 = REG_X20; // Saved register 4
pub const REG_S5: u64 = REG_X21; // Saved register 5
pub const REG_S6: u64 = REG_X22; // Saved register 6
pub const REG_S7: u64 = REG_X23; // Saved register 7
pub const REG_S8: u64 = REG_X24; // Saved register 8
pub const REG_S9: u64 = REG_X25; // Saved register 9
pub const REG_S10: u64 = REG_X26; // Saved register 10
pub const REG_S11: u64 = REG_X27; // Saved register 11
pub const REG_T3: u64 = REG_X28; // Temporary register 3
pub const REG_T4: u64 = REG_X29; // Temporary register 4
pub const REG_T5: u64 = REG_X30; // Temporary register 5
pub const REG_T6: u64 = REG_X31; // Temporary register 6

pub const REGS_IN_MAIN_FROM: usize = 1; // First non-zero register in main trace
pub const REGS_IN_MAIN_TO: usize = 31; // Last non-zero register in main trace
pub const REGS_IN_MAIN: usize = REGS_IN_MAIN_TO - REGS_IN_MAIN_FROM + 1;
pub const REGS_IN_MAIN_TOTAL_NUMBER: usize = 32; // Total number of INTEGER registers in main, including the zero register

// Total register counts (including both integer and floating point)
pub const TOTAL_INTEGER_REGISTERS: usize = 32; // x0-x31
pub const TOTAL_FLOATING_POINT_REGISTERS: usize = 32; // f0-f31
pub const TOTAL_ALL_REGISTERS: usize = TOTAL_INTEGER_REGISTERS + TOTAL_FLOATING_POINT_REGISTERS; // 64 total

// Floating Point CSR addresses (RISC-V standard)
pub const CSR_FFLAGS: u32 = 0x001;  // Floating-point exception flags
pub const CSR_FRM: u32 = 0x002;     // Floating-point rounding mode  
pub const CSR_FCSR: u32 = 0x003;    // Floating-point control and status register

// Floating Point Rounding Modes (frm field values)
pub const FRM_RNE: u32 = 0b000;  // Round to Nearest, ties to Even
pub const FRM_RTZ: u32 = 0b001;  // Round towards Zero
pub const FRM_RDN: u32 = 0b010;  // Round Down (towards negative infinity)
pub const FRM_RUP: u32 = 0b011;  // Round Up (towards positive infinity)  
pub const FRM_RMM: u32 = 0b100;  // Round to Nearest, ties to Max Magnitude
pub const FRM_DYN: u32 = 0b111;  // Dynamic rounding mode (use frm CSR)

// Floating Point Exception Flags (fflags field bits)
pub const FFLAGS_NX: u32 = 1 << 0;  // Inexact
pub const FFLAGS_UF: u32 = 1 << 1;  // Underflow
pub const FFLAGS_OF: u32 = 1 << 2;  // Overflow
pub const FFLAGS_DZ: u32 = 1 << 3;  // Divide by Zero
pub const FFLAGS_NV: u32 = 1 << 4;  // Invalid Operation
