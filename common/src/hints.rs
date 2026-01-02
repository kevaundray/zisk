//! Hints for ZisK Precompiles stream processing
//!
//! This module provides functionality for parsing precompile hints
//! that are received as a stream of `u64` values. Hints are used to provide preprocessed
//! data to precompile operations in the ZisK zkVM.
//!
//! # Hint Format
//!
//! Each hint consists of:
//! - A **header** (`u64`): Contains the hint type (upper 32 bits) and data length (lower 32 bits)
//! - **Data** (`[u64; length]`): The hint payload, where `length` is specified in the header
//!
//! ```text
//! ┌─────────────────────────────────────────────────────────────┐
//! │                         Header (u64)                        │
//! ├·····························································┤
//! │      Hint Code (32 bits)           Length (32 bits).        │
//! ├─────────────────────────────────────────────────────────────┤
//! │                        Data[0] (u64)                        │
//! ├─────────────────────────────────────────────────────────────┤
//! │                        Data[1] (u64)                        │
//! ├─────────────────────────────────────────────────────────────┤
//! │                             ...                             │
//! ├─────────────────────────────────────────────────────────────┤
//! │                     Data[length-1] (u64)                    │
//! └─────────────────────────────────────────────────────────────┘
//!
//! - Hint Code — Control code or Data Hint Type
//! - Length — Number of following u64 data words
//!
//! ## Hint Type Layout
//!
//! ### Control codes
//!
//! The following control codes are defined:
//! - `0x00` (START): Reset processor state and global sequence.
//! - `0x01` (END): Wait until completion of all pending hints.
//! - `0x02` (CANCEL): Cancel current stream and stop processing further hints.
//! - `0x03` (ERROR): Indicate an error has occurred; stop processing further hints.
//!
//! Control codes are for control only and do not have any associated data (Length should be zero).
//!
//! ### Data Hint Types:
//! - `0x04` (`Noop`): Pass-through data
//! - `0x05` (`EcRecover`): ECRECOVER inputs (currently returns empty)
//! ```

use std::fmt::Display;

use anyhow::Result;

/// Control code variants for stream control.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(u32)]
pub enum CtrlCode {
    /// Reset processor state and global sequence.
    Start = 0x00,
    /// Wait until completion of all pending hints.
    End = 0x01,
    /// Cancel current stream and stop processing.
    Cancel = 0x02,
    /// Signal error and stop processing.
    Error = 0x03,
}

impl Display for CtrlCode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let name = match self {
            CtrlCode::Start => "CTRL_START",
            CtrlCode::End => "CTRL_END",
            CtrlCode::Cancel => "CTRL_CANCEL",
            CtrlCode::Error => "CTRL_ERROR",
        };
        write!(f, "{}", name)
    }
}

/// Built-in hint type variants.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(u32)]
pub enum BuiltInHint {
    /// Pass-through hint type.
    /// When a hint has this type, the processor simply passes through the data
    /// without any additional computation.
    Noop = 0x04,
    /// Ecrecover precompile hint type.
    EcRecover = 0x05,
    ///  Modular reduction of a 256-bit integer hint type.
    RedMod256 = 0x06,
    /// Modular addition of 256-bit integers hint type.
    AddMod256 = 0x07,
    /// Modular multiplication of 256-bit integers hint type.
    MulMod256 = 0x08,
    /// Division and remainder of 256-bit integers hint type.
    DivRem256 = 0x09,
    /// Wrapping exponentiation of 256-bit integers hint type.
    WPow256 = 0x0A,
    /// Overflowing multiplication of 256-bit integers hint type.
    OMul256 = 0x0B,
    /// Wrapping multiplication of 256-bit integers hint type.
    WMul256 = 0x0C,
}

impl Display for BuiltInHint {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let name = match self {
            BuiltInHint::Noop => "NOOP",
            BuiltInHint::EcRecover => "ECRECOVER",
            BuiltInHint::RedMod256 => "REDMOD256",
            BuiltInHint::AddMod256 => "ADDMOD256",
            BuiltInHint::MulMod256 => "MULMOD256",
            BuiltInHint::DivRem256 => "DIVREM256",
            BuiltInHint::WPow256 => "WPOW256",
            BuiltInHint::OMul256 => "OMUL256",
            BuiltInHint::WMul256 => "WMUL256",
        };
        write!(f, "{}", name)
    }
}

/// Hint code representing either a control code or built-in hint type.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(u32)]
pub enum HintCode {
    /// Control code for stream management.
    Ctrl(CtrlCode),
    /// Built-in hint type.
    BuiltIn(BuiltInHint),
}

impl Display for HintCode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            HintCode::Ctrl(ctrl) => write!(f, "{}", ctrl),
            HintCode::BuiltIn(builtin) => write!(f, "{}", builtin),
        }
    }
}

impl TryFrom<u32> for HintCode {
    type Error = anyhow::Error;

    fn try_from(value: u32) -> Result<Self> {
        match value {
            0x00 => Ok(HintCode::Ctrl(CtrlCode::Start)),
            0x01 => Ok(HintCode::Ctrl(CtrlCode::End)),
            0x02 => Ok(HintCode::Ctrl(CtrlCode::Cancel)),
            0x03 => Ok(HintCode::Ctrl(CtrlCode::Error)),
            0x04 => Ok(HintCode::BuiltIn(BuiltInHint::Noop)),
            0x05 => Ok(HintCode::BuiltIn(BuiltInHint::EcRecover)),
            0x06 => Ok(HintCode::BuiltIn(BuiltInHint::RedMod256)),
            0x07 => Ok(HintCode::BuiltIn(BuiltInHint::AddMod256)),
            0x08 => Ok(HintCode::BuiltIn(BuiltInHint::MulMod256)),
            0x09 => Ok(HintCode::BuiltIn(BuiltInHint::DivRem256)),
            0x0A => Ok(HintCode::BuiltIn(BuiltInHint::WPow256)),
            0x0B => Ok(HintCode::BuiltIn(BuiltInHint::OMul256)),
            0x0C => Ok(HintCode::BuiltIn(BuiltInHint::WMul256)),
            _ => Err(anyhow::anyhow!("Invalid hint code: {:#x}", value)),
        }
    }
}

impl HintCode {
    /// Convert HintCode to its u32 discriminant value.
    #[inline]
    pub const fn to_u32(self) -> u32 {
        match self {
            HintCode::Ctrl(CtrlCode::Start) => 0x00,
            HintCode::Ctrl(CtrlCode::End) => 0x01,
            HintCode::Ctrl(CtrlCode::Cancel) => 0x02,
            HintCode::Ctrl(CtrlCode::Error) => 0x03,
            HintCode::BuiltIn(BuiltInHint::Noop) => 0x04,
            HintCode::BuiltIn(BuiltInHint::EcRecover) => 0x05,
            HintCode::BuiltIn(BuiltInHint::RedMod256) => 0x06,
            HintCode::BuiltIn(BuiltInHint::AddMod256) => 0x07,
            HintCode::BuiltIn(BuiltInHint::MulMod256) => 0x08,
            HintCode::BuiltIn(BuiltInHint::DivRem256) => 0x09,
            HintCode::BuiltIn(BuiltInHint::WPow256) => 0x0A,
            HintCode::BuiltIn(BuiltInHint::OMul256) => 0x0B,
            HintCode::BuiltIn(BuiltInHint::WMul256) => 0x0C,
        }
    }
}

/// Represents a single precompile hint parsed from a `u64` slice.
///
/// A hint consists of a type identifier and associated data. The hint type
/// determines how the data should be processed by the [`PrecompileHintsProcessor`].
pub struct PrecompileHint {
    /// The type of hint, determining how the data should be processed.
    pub hint_code: HintCode,
    /// The hint payload data.
    pub data: Vec<u64>,
}

impl std::fmt::Debug for PrecompileHint {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let data_display = if self.data.len() <= 10 {
            format!("{:?}", self.data)
        } else {
            format!("{:?}... ({} more)", &self.data[..10], self.data.len() - 10)
        };
        f.debug_struct("PrecompileHint")
            .field("hint_type", &self.hint_code)
            .field("data", &data_display)
            .finish()
    }
}

impl PrecompileHint {
    /// Parses a [`PrecompileHint`] from a slice of `u64` values at the given index.
    ///
    /// # Arguments
    ///
    /// * `slice` - The source slice containing concatenated hints
    /// * `idx` - The index where the hint header starts
    ///
    /// # Returns
    ///
    /// * `Ok(PrecompileHint)` - Successfully parsed hint
    /// * `Err` - If the slice is too short or the index is out of bounds
    #[inline(always)]
    pub fn from_u64_slice(slice: &[u64], idx: usize) -> Result<Self> {
        if slice.is_empty() || idx >= slice.len() {
            return Err(anyhow::anyhow!("Slice too short or index out of bounds"));
        }

        let header = slice[idx];
        let length = (header & 0xFFFFFFFF) as u32;

        if slice.len() < idx + length as usize + 1 {
            return Err(anyhow::anyhow!(
                "Slice too short for hint data: expected {}, got {}",
                length,
                slice.len() - idx - 1
            ));
        }

        let hint_code_32 = (header >> 32) as u32;
        let hint_code = HintCode::try_from(hint_code_32)?;

        // Create a new Vec with the hint data.
        let data = slice[idx + 1..idx + length as usize + 1].to_vec();

        Ok(PrecompileHint { hint_code, data })
    }
}
