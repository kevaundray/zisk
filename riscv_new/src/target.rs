//! RISC-V target configuration
// TODO: Remove this as the code will be fixed re what it supports.
use std::fmt;

/// RISC-V instruction set extensions
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Extension {
    /// RV32I - Base integer instruction set
    RV32I,
    /// RV64I - 64-bit extensions to base
    RV64I,
    /// RV32M - Integer multiply/divide
    RV32M,
    /// RV64M - 64-bit multiply/divide  
    RV64M,
    /// RV32A - Atomic instructions
    RV32A,
    /// RV64A - 64-bit atomic instructions
    RV64A,
    /// Zicsr - Control and Status Register instructions
    Zicsr,
    /// Zifencei - Instruction-fetch fence
    Zifencei,
    /// Zicntr - Counter extension (performance counters)
    Zicntr,
    /// Zihpm - Hardware Performance Monitors extension
    Zihpm,
    /// RV32F - Single-precision floating point
    RV32F,
    /// RV64F - 64-bit single-precision floating point
    RV64F,
    /// RV32D - Double-precision floating point  
    RV32D,
    /// RV64D - 64-bit double-precision floating point
    RV64D,
    /// RVC - Compressed instruction extension
    RVC,
}

impl fmt::Display for Extension {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Extension::RV32I => write!(f, "RV32I (Base Integer)"),
            Extension::RV64I => write!(f, "RV64I (64-bit Extensions)"),
            Extension::RV32M => write!(f, "RV32M (Multiply/Divide)"),
            Extension::RV64M => write!(f, "RV64M (64-bit Multiply/Divide)"),
            Extension::RV32A => write!(f, "RV32A (Atomic)"),
            Extension::RV64A => write!(f, "RV64A (64-bit Atomic)"),
            Extension::Zicsr => write!(f, "Zicsr (CSR Instructions)"),
            Extension::Zifencei => write!(f, "Zifencei (Instruction Fence)"),
            Extension::Zicntr => write!(f, "Zicntr (Counter Extension)"),
            Extension::Zihpm => write!(f, "Zihpm (Hardware Performance Monitors)"),
            Extension::RV32F => write!(f, "RV32F (Single-precision Floating Point)"),
            Extension::RV64F => write!(f, "RV64F (64-bit Single-precision Floating Point)"),
            Extension::RV32D => write!(f, "RV32D (Double-precision Floating Point)"),
            Extension::RV64D => write!(f, "RV64D (64-bit Double-precision Floating Point)"),
            Extension::RVC => write!(f, "RVC (Compressed)"),
        }
    }
}

/// RISC-V target configuration using builder pattern
#[derive(Debug, Clone, PartialEq)]
pub struct Target {
    /// Base instruction set
    /// TODO: this is always enabled
    i: bool,
    /// Multiply/divide extension
    m: bool,
    /// Atomic extension
    a: bool,
    /// Compressed instruction extension
    c: bool,
    /// 64-bit extension
    i64: bool,
    /// CSR extension
    zicsr: bool,
    /// Instruction fence extension
    zifencei: bool,
    /// Counter extension
    zicntr: bool,
    /// Hardware Performance Monitors extension
    zihpm: bool,
    /// Single-precision floating point extension
    f: bool,
    /// Double-precision floating point extension
    d: bool,
}

impl Target {
    /// Create a new target with just RV32I base
    pub const fn new() -> Self {
        Self {
            i: true,
            m: false,
            a: false,
            c: false,
            i64: false,
            zicsr: false,
            zifencei: false,
            zicntr: false,
            zihpm: false,
            f: false,
            d: false,
        }
    }

    /// Enable multiply/divide extension (M)
    pub const fn with_m(mut self) -> Self {
        self.m = true;
        self
    }

    /// Enable atomic extension (A)
    pub const fn with_a(mut self) -> Self {
        self.a = true;
        self
    }

    /// Enable compressed instruction extension (C)
    pub const fn with_c(mut self) -> Self {
        self.c = true;
        self
    }

    /// Enable 64-bit extension (RV64I)
    pub const fn with_64bit(mut self) -> Self {
        self.i64 = true;
        self
    }

    /// Enable CSR extension (Zicsr)
    pub const fn with_zicsr(mut self) -> Self {
        self.zicsr = true;
        self
    }

    /// Enable instruction fence extension (Zifencei)
    pub const fn with_zifencei(mut self) -> Self {
        self.zifencei = true;
        self
    }

    /// Enable counter extension (Zicntr)
    pub const fn with_zicntr(mut self) -> Self {
        self.zicntr = true;
        self
    }

    /// Enable hardware performance monitors extension (Zihpm)
    pub const fn with_zihpm(mut self) -> Self {
        self.zihpm = true;
        self
    }

    /// Enable single-precision floating point extension (F)
    pub const fn with_f(mut self) -> Self {
        self.f = true;
        self
    }

    /// Enable double-precision floating point extension (D)
    /// Note: D extension requires F extension
    pub const fn with_d(mut self) -> Self {
        self.f = true; // D implies F
        self.d = true;
        self
    }

    /// Create RV32IMC target
    pub fn rv32imc() -> Self {
        Self::new().with_m().with_c()
    }

    /// Create RV64IMAC target (common 64-bit configuration)
    pub const fn rv64imac() -> Self {
        Self::new().with_64bit().with_m().with_a().with_c()
    }

    /// Create RV64GC target (official: RV64IMAFD_Zicsr_Zifencei + C)
    pub const fn rv64gc() -> Self {
        Self::new()
            .with_64bit()
            .with_m()
            .with_a()
            .with_f()
            .with_d()
            .with_c()
            .with_zicsr()
            .with_zifencei()
    }

    /// Check if an extension is supported
    pub const fn supports_extension(&self, extension: Extension) -> bool {
        match extension {
            Extension::RV32I => self.i,
            Extension::RV64I => self.i && self.i64,
            Extension::RV32M => self.m,
            Extension::RV64M => self.m && self.i64,
            Extension::RV32A => self.a,
            Extension::RV64A => self.a && self.i64,
            // TODO: Add these Z extensions into decoder
            Extension::Zicsr => self.zicsr,
            Extension::Zifencei => self.zifencei,
            Extension::Zicntr => self.zicntr,
            Extension::Zihpm => self.zihpm,
            Extension::RV32F => self.f,
            Extension::RV64F => self.f && self.i64,
            Extension::RV32D => self.d,
            Extension::RV64D => self.d && self.i64,
            Extension::RVC => self.c,
        }
    }

    /// Get a string representation of the target
    pub fn target_string(&self) -> String {
        let mut result = if self.i64 { "RV64".to_string() } else { "RV32".to_string() };

        if self.i {
            result.push('I');
        }
        if self.m {
            result.push('M');
        }
        if self.a {
            result.push('A');
        }
        if self.f {
            result.push('F');
        }
        if self.d {
            result.push('D');
        }
        if self.c {
            result.push('C');
        }

        let mut extensions = Vec::new();
        if self.zicsr {
            extensions.push("Zicsr");
        }
        if self.zifencei {
            extensions.push("Zifencei");
        }
        if self.zicntr {
            extensions.push("Zicntr");
        }
        if self.zihpm {
            extensions.push("Zihpm");
        }

        if !extensions.is_empty() {
            result.push('_');
            result.push_str(&extensions.join("_"));
        }

        result
    }

    pub const fn compressed_enabled(&self) -> bool {
        self.c
    }

    /// Get all enabled extensions
    pub fn enabled_extensions(&self) -> Vec<Extension> {
        let mut extensions = Vec::new();

        if self.i {
            extensions.push(Extension::RV32I);
        }
        if self.i && self.i64 {
            // Note: this and is here because its not possible to have RV64I without RV32I
            extensions.push(Extension::RV64I);
        }
        if self.m {
            extensions.push(Extension::RV32M);
        }
        if self.m && self.i64 {
            extensions.push(Extension::RV64M);
        }
        if self.a {
            extensions.push(Extension::RV32A);
        }
        if self.a && self.i64 {
            extensions.push(Extension::RV64A);
        }
        if self.f {
            extensions.push(Extension::RV32F);
        }
        if self.f && self.i64 {
            extensions.push(Extension::RV64F);
        }
        if self.d {
            extensions.push(Extension::RV32D);
        }
        if self.d && self.i64 {
            extensions.push(Extension::RV64D);
        }
        if self.c {
            extensions.push(Extension::RVC);
        }
        if self.zicsr {
            extensions.push(Extension::Zicsr);
        }
        if self.zifencei {
            extensions.push(Extension::Zifencei);
        }
        if self.zicntr {
            extensions.push(Extension::Zicntr);
        }
        if self.zihpm {
            extensions.push(Extension::Zihpm);
        }

        extensions
    }
}

impl Default for Target {
    fn default() -> Self {
        Self::new()
    }
}

impl std::fmt::Display for Target {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.target_string())
    }
}
