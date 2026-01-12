//! * Provides a context to execute a set of Zisk instructions.
//! * The context contains the state of the Zisk processor, modified by the execution of every
//!   instruction.
//! * The state includes: memory, registers (a, b, c, flag, sp), program counter (pc), step and a
//!   flag to mark the end of the program execution.

use crate::{
    Mem, FCALL_PARAMS_MAX_SIZE, FCALL_RESULT_MAX_SIZE, REGS_IN_MAIN_TOTAL_NUMBER, ROM_ENTRY,
};

const PARAMS_MAX_SIZE: usize = 4;

/// Zisk precompiled emulation mode
#[derive(Debug, Default, PartialEq, Eq)]
pub enum EmulationMode {
    #[default]
    Mem,
    GenerateMemReads,
    ConsumeMemReads,
}

/// Zisk precompiled instruction context.
/// Stores the input data (of the size expected by the precompiled components) and the output data.
/// If the precompiled component finds input_data not empty, it should use this data instead of
/// reading it from memory
#[derive(Debug, Default)]
pub struct PrecompiledInstContext {
    /// Step
    pub step: u64,

    /// Precompiled input data address
    // pub input_data_address: u64,
    /// Precompiled input data
    pub input_data: Vec<u64>,

    /// Precompiled output data address
    // pub output_data_address: u64,
    /// Precompiled output data
    pub output_data: Vec<u64>,
}

/// Zisk fcall instruction context.
/// Stores the fcall arguments data and the result data.
#[derive(Debug)]
pub struct FcallInstContext {
    /// Fcall parameters data
    /// Maximum size is FCALL_PARAMS_MAX_SIZE u64s
    pub parameters: [u64; FCALL_PARAMS_MAX_SIZE],

    /// Indicates how many parameter u64s contain valid data
    pub parameters_size: u64,

    /// Fcall result data
    /// Maximum size is FCALL_RESULT_MAX_SIZE u64s
    pub result: [u64; FCALL_RESULT_MAX_SIZE],

    /// Indicates how many result u64s contain valid data
    pub result_size: u64,

    /// Indicates how many result u64s have been read using fcall_get()
    pub result_got: u64,
}

impl Default for FcallInstContext {
    /// Default fcall instruction context constructor
    fn default() -> Self {
        Self {
            parameters: [0; FCALL_PARAMS_MAX_SIZE],
            parameters_size: 0,
            result: [0; FCALL_RESULT_MAX_SIZE],
            result_size: 0,
            result_got: 0,
        }
    }
}

/// Zisk param instruction context, these instructions are used to pass extra parameters to
/// precompiles. Currently precompiles can receive up to 2 parameters directly in instruction call,
/// but if this precompile needs more parameters use these instructions to pass them. It's important
/// to note that these parameters must be called in the instructions immediately before, because when
/// precompiles prove them they use step - 1, step - 2 and so on.
///
/// Stores the precompile arguments.
#[derive(Debug)]
pub struct ParamInstContext {
    /// Maximum size is PARAMS_MAX_SIZE u64s
    pub parameters: [u64; PARAMS_MAX_SIZE],

    /// Indicates how many parameter u64s contain valid data
    pub parameters_size: usize,

    /// Indicates the max step for these parameters
    pub step_limit: u64,
}

impl Default for ParamInstContext {
    /// Default param instruction context constructor
    fn default() -> Self {
        Self { parameters: [0; PARAMS_MAX_SIZE], parameters_size: 0, step_limit: 0 }
    }
}

impl ParamInstContext {
    /// Adds a single param.
    pub fn add_param(&mut self, value: u64, step: u64) {
        if step > self.step_limit {
            self.step_limit = step + PARAMS_MAX_SIZE as u64 + 1;
            self.parameters_size = 0;
        }
        if self.parameters_size >= PARAMS_MAX_SIZE {
            panic!(
                "ERROR: no space for one more parameter ({}/{} step_limit:{})",
                self.parameters_size, PARAMS_MAX_SIZE, self.step_limit
            );
        }
        self.parameters[self.parameters_size] = value;
        self.parameters_size += 1;
    }
    /// Adds multiple params (double normally).
    pub fn add_params(&mut self, values: &[u64], step: u64) {
        if step > self.step_limit {
            self.step_limit = PARAMS_MAX_SIZE as u64 + 1;
            self.parameters_size = 0;
        }
        if self.parameters_size + values.len() > PARAMS_MAX_SIZE {
            panic!(
                "ERROR: no space for {} more parameters ({}/{} step_limit:{})",
                values.len(),
                self.parameters_size,
                PARAMS_MAX_SIZE,
                self.step_limit
            );
        }
        for value in values {
            self.parameters[self.parameters_size] = *value;
            self.parameters_size += 1;
        }
    }
    /// Clears params.
    pub fn clear(&mut self) {
        self.step_limit = 0;
        self.parameters_size = 0;
    }
    /// Gets a param by index.
    pub fn get_param(&self, index: usize) -> Option<u64> {
        if index < self.parameters_size {
            Some(self.parameters[index])
        } else {
            None
        }
    }
}

#[derive(Debug)]
/// ZisK instruction context data container, storing the state of the execution
pub struct InstContext {
    /// Memory, including several read-only sections and one read-write section (input data)
    /// This memory is initialized before running the program with the input data, and modified by
    /// the program instructions during the execution.  The RW data that has not been previously
    /// written is read as zero
    pub mem: Mem,

    /// Current value of register a
    pub a: u64,
    /// Current value of register b
    pub b: u64,
    /// Current value of register c
    pub c: u64,
    /// Current value of register flag
    pub flag: bool,

    /// Current value of register sp
    pub sp: u64,

    /// Current value of ROM program execution address, i.e. program counter (pc)
    pub pc: u64,

    /// Current execution step: 0, 1, 2...
    pub step: u64,

    /// End flag, set to true only by the last instruction to execute
    pub end: bool,

    /// Error flag, set to true if an error occurs during execution, e.g. halt instruction due to
    /// a 0x0000 instruction
    pub error: bool,

    /// Registers
    pub regs: [u64; REGS_IN_MAIN_TOTAL_NUMBER],

    /// Precompiled emulation mode
    pub emulation_mode: EmulationMode,

    /// Precompiled data
    pub precompiled: PrecompiledInstContext,

    /// Fcall data
    pub fcall: FcallInstContext,

    /// Params data
    pub params: ParamInstContext,

    /// DataExt 64 bytes size. With this information it is possible to specify which variable part of the minimal trace
    /// is associated with the current instruction. Used by DMA precompile.
    pub data_ext_len: usize,
}

/// RisK instruction context implementation
impl InstContext {
    /// RisK instruction context constructor
    pub fn new() -> InstContext {
        InstContext {
            mem: Mem::default(),
            a: 0,
            b: 0,
            c: 0,
            flag: false,
            sp: 0,
            pc: ROM_ENTRY,
            step: 0,
            end: false,
            error: false,
            regs: [0; REGS_IN_MAIN_TOTAL_NUMBER],
            emulation_mode: EmulationMode::default(),
            precompiled: PrecompiledInstContext::default(),
            fcall: FcallInstContext::default(),
            params: ParamInstContext::default(),
            data_ext_len: 0,
        }
    }

    /// Creates a human-readable string describing the instruction context, for debugging purposes
    pub fn to_text(&self) -> String {
        let s = format! {"a={:x} b={:x} c={:x} flag={} sp={} pc={} step={} end={}", self.a, self.b, self.c, self.flag, self.sp, self.pc, self.step, self.end};
        s
    }
}

impl Default for InstContext {
    /// Default instruction context constructor
    fn default() -> Self {
        Self::new()
    }
}
