//! Standard RISC-V instruction decoders for each format type

use std::collections::HashMap;
use crate::instruction::{DecodedInstruction, DecodeResult, DecodeError, InstructionFormat, Opcode};
use super::{StandardInstructionDecoder, FieldExtractor, utils::*, XLen};

/// Decoder for R-type instructions (register-register operations)
pub struct RTypeDecoder {
    /// Mapping from (funct3, funct7) to instruction mnemonic
    mnemonics: HashMap<(u8, u8), &'static str>,
}

impl RTypeDecoder {
    pub fn new() -> Self {
        let mut mnemonics = HashMap::new();
        
        // Standard R-type instructions
        mnemonics.insert((0, 0), "add");
        mnemonics.insert((0, 32), "sub");
        mnemonics.insert((1, 0), "sll");
        mnemonics.insert((2, 0), "slt");
        mnemonics.insert((3, 0), "sltu");
        mnemonics.insert((4, 0), "xor");
        mnemonics.insert((5, 0), "srl");
        mnemonics.insert((5, 32), "sra");
        mnemonics.insert((6, 0), "or");
        mnemonics.insert((7, 0), "and");
        
        // RV32M extension
        mnemonics.insert((0, 1), "mul");
        mnemonics.insert((1, 1), "mulh");
        mnemonics.insert((2, 1), "mulhsu");
        mnemonics.insert((3, 1), "mulhu");
        mnemonics.insert((4, 1), "div");
        mnemonics.insert((5, 1), "divu");
        mnemonics.insert((6, 1), "rem");
        mnemonics.insert((7, 1), "remu");
        
        Self { mnemonics }
    }
    
    /// Create a decoder specifically for RV64I word operations (OP-32 opcode)
    pub fn new_rv64_word() -> Self {
        let mut mnemonics = HashMap::new();
        
        // RV64I word operations (32-bit operations in 64-bit mode)
        mnemonics.insert((0, 0), "addw");
        mnemonics.insert((0, 32), "subw");
        mnemonics.insert((1, 0), "sllw");
        mnemonics.insert((5, 0), "srlw");
        mnemonics.insert((5, 32), "sraw");
        
        // RV64M word operations
        mnemonics.insert((0, 1), "mulw");
        mnemonics.insert((4, 1), "divw");
        mnemonics.insert((5, 1), "divuw");
        mnemonics.insert((6, 1), "remw");
        mnemonics.insert((7, 1), "remuw");
        
        Self { mnemonics }
    }
}

impl StandardInstructionDecoder for RTypeDecoder {
    fn format(&self) -> InstructionFormat {
        InstructionFormat::R
    }
    
    fn decode(&self, inst: u32) -> DecodeResult<DecodedInstruction> {
        let opcode = Opcode::try_from(inst.opcode())?;
        let funct3 = inst.funct3();
        let funct7 = inst.funct7();
        let mnemonic = self.get_mnemonic(funct3, funct7)?;
        
        Ok(DecodedInstruction::RType {
            raw: inst,
            opcode,
            mnemonic,
            rd: inst.rd(),
            rs1: inst.rs1(),
            rs2: inst.rs2(),
            funct3,
            funct7,
        })
    }
    
    fn get_mnemonic(&self, funct3: u8, funct7: u8) -> DecodeResult<String> {
        self.mnemonics
            .get(&(funct3, funct7))
            .map(|&s| s.to_string())
            .ok_or(DecodeError::InvalidFunct(funct3, funct7))
    }
}

/// Decoder for I-type instructions (immediate and load operations)
pub struct ITypeDecoder {
    /// Mapping from funct3 to instruction mnemonic for different opcodes
    load_mnemonics: HashMap<u8, &'static str>,
    imm_mnemonics: HashMap<u8, &'static str>,
    xlen: XLen,
}

impl ITypeDecoder {
    pub fn new(xlen: XLen) -> Self {
        let mut load_mnemonics = HashMap::new();
        load_mnemonics.insert(0, "lb");
        load_mnemonics.insert(1, "lh");
        load_mnemonics.insert(2, "lw");
        load_mnemonics.insert(3, "ld");
        load_mnemonics.insert(4, "lbu");
        load_mnemonics.insert(5, "lhu");
        load_mnemonics.insert(6, "lwu");
        
        let mut imm_mnemonics = HashMap::new();
        imm_mnemonics.insert(0, "addi");
        imm_mnemonics.insert(2, "slti");
        imm_mnemonics.insert(3, "sltiu");
        imm_mnemonics.insert(4, "xori");
        imm_mnemonics.insert(6, "ori");
        imm_mnemonics.insert(7, "andi");
        
        // RV64I 32-bit word operations (OP-IMM-32)
        // Note: These will be distinguished by opcode in decode method
        
        Self { load_mnemonics, imm_mnemonics, xlen }
    }
}

impl StandardInstructionDecoder for ITypeDecoder {
    fn format(&self) -> InstructionFormat {
        InstructionFormat::I
    }
    
    fn decode(&self, inst: u32) -> DecodeResult<DecodedInstruction> {
        let opcode = Opcode::try_from(inst.opcode())?;
        let funct3 = inst.funct3();
        let funct7 = inst.funct7();
        
        // Get the mnemonic based on the opcode and funct3/funct7
        let mnemonic = match opcode {
            Opcode::Load => {
                self.load_mnemonics.get(&funct3)
                    .map(|&s| s.to_string())
                    .ok_or(DecodeError::InvalidFunct(funct3, funct7))?
            },
            Opcode::OpImm => {
                // Immediate shifts use funct3 plus bit 30 (in imm upper bits) to choose arithmetic vs logical
                match funct3 {
                    1 => {
                        // slli: validate shamt width per XLEN (RV32 => 5 bits, RV64 => 6 bits)
                        let shamt = ((inst >> 20) & 0x3F) as u32;
                        if self.xlen == XLen::X32 && (shamt & 0x20) != 0 {
                            return Err(DecodeError::InvalidFunct(funct3, funct7));
                        }
                        "slli".to_string()
                    },
                    5 => {
                        let is_arith = ((inst >> 30) & 1) == 1; // SRAI when bit 30 set
                        // Validate shamt width (same rule as slli)
                        let shamt = ((inst >> 20) & 0x3F) as u32;
                        if self.xlen == XLen::X32 && (shamt & 0x20) != 0 {
                            return Err(DecodeError::InvalidFunct(funct3, funct7));
                        }
                        if is_arith { "srai".to_string() } else { "srli".to_string() }
                    }
                    _ => self.imm_mnemonics.get(&funct3)
                        .map(|&s| s.to_string())
                        .ok_or(DecodeError::InvalidFunct(funct3, funct7))?,
                }
            },
            Opcode::OpImm32 => {
                // RV64I word operations  
                match funct3 {
                    0 => "addiw".to_string(),
                    1 => {
                        if funct7 == 0 {
                            "slliw".to_string()
                        } else {
                            return Err(DecodeError::InvalidFunct(funct3, funct7));
                        }
                    },
                    5 => {
                        match funct7 {
                            0 => "srliw".to_string(),
                            32 => "sraiw".to_string(),
                            _ => return Err(DecodeError::InvalidFunct(funct3, funct7)),
                        }
                    },
                    _ => return Err(DecodeError::InvalidFunct(funct3, funct7)),
                }
            },
            Opcode::Jalr => {
                if funct3 == 0 {
                    "jalr".to_string()
                } else {
                    return Err(DecodeError::InvalidFunct(funct3, funct7));
                }
            },
            _ => return Err(DecodeError::InvalidFormat),
        };
        
        let mut imm = extract_i_immediate(inst);
        let mut resolved_funct7 = 0;
        
        // For shift instructions, limit immediate width appropriately and set funct7 for inspection/roundtrips
        if matches!(funct3, 1 | 5) && matches!(opcode, Opcode::OpImm | Opcode::OpImm32) {
            if opcode == Opcode::OpImm32 {
                imm &= 0x1F; // word ops use 5-bit shamt
            } else {
                // Tailor to XLEN: RV32 => 5 bits, RV64 => 6 bits
                imm &= if self.xlen == XLen::X32 { 0x1F } else { 0x3F };
            }
            resolved_funct7 = funct7;
        }
        
        Ok(DecodedInstruction::IType {
            raw: inst,
            opcode,
            mnemonic,
            rd: inst.rd(),
            rs1: inst.rs1(),
            imm,
            funct3,
            funct7: resolved_funct7,
        })
    }
    
    fn get_mnemonic(&self, _funct3: u8, _funct7: u8) -> DecodeResult<String> {
        // This is handled in decode() method based on opcode context
        Err(DecodeError::InvalidFormat)
    }
}

/// Decoder for S-type instructions (store operations)
pub struct STypeDecoder {
    mnemonics: HashMap<u8, &'static str>,
}

impl STypeDecoder {
    pub fn new() -> Self {
        let mut mnemonics = HashMap::new();
        mnemonics.insert(0, "sb");
        mnemonics.insert(1, "sh");
        mnemonics.insert(2, "sw");
        mnemonics.insert(3, "sd");
        
        Self { mnemonics }
    }
}

impl StandardInstructionDecoder for STypeDecoder {
    fn format(&self) -> InstructionFormat {
        InstructionFormat::S
    }
    
    fn decode(&self, inst: u32) -> DecodeResult<DecodedInstruction> {
        let opcode = Opcode::try_from(inst.opcode())?;
        let funct3 = inst.funct3();
        let mnemonic = self.get_mnemonic(funct3, 0)?;
        
        Ok(DecodedInstruction::SType {
            raw: inst,
            opcode,
            mnemonic,
            rs1: inst.rs1(),
            rs2: inst.rs2(),
            imm: extract_s_immediate(inst),
            funct3,
        })
    }
    
    fn get_mnemonic(&self, funct3: u8, _funct7: u8) -> DecodeResult<String> {
        self.mnemonics
            .get(&funct3)
            .map(|&s| s.to_string())
            .ok_or(DecodeError::InvalidFunct(funct3, 0))
    }
}

/// Decoder for B-type instructions (branch operations)
pub struct BTypeDecoder {
    mnemonics: HashMap<u8, &'static str>,
}

impl BTypeDecoder {
    pub fn new() -> Self {
        let mut mnemonics = HashMap::new();
        mnemonics.insert(0, "beq");
        mnemonics.insert(1, "bne");
        mnemonics.insert(4, "blt");
        mnemonics.insert(5, "bge");
        mnemonics.insert(6, "bltu");
        mnemonics.insert(7, "bgeu");
        
        Self { mnemonics }
    }
}

impl StandardInstructionDecoder for BTypeDecoder {
    fn format(&self) -> InstructionFormat {
        InstructionFormat::B
    }
    
    fn decode(&self, inst: u32) -> DecodeResult<DecodedInstruction> {
        let opcode = Opcode::try_from(inst.opcode())?;
        let funct3 = inst.funct3();
        let mnemonic = self.get_mnemonic(funct3, 0)?;
        
        Ok(DecodedInstruction::BType {
            raw: inst,
            opcode,
            mnemonic,
            rs1: inst.rs1(),
            rs2: inst.rs2(),
            imm: extract_b_immediate(inst),
            funct3,
        })
    }
    
    fn get_mnemonic(&self, funct3: u8, _funct7: u8) -> DecodeResult<String> {
        self.mnemonics
            .get(&funct3)
            .map(|&s| s.to_string())
            .ok_or(DecodeError::InvalidFunct(funct3, 0))
    }
}

/// Decoder for U-type instructions (upper immediate operations)
pub struct UTypeDecoder;

impl StandardInstructionDecoder for UTypeDecoder {
    fn format(&self) -> InstructionFormat {
        InstructionFormat::U
    }
    
    fn decode(&self, inst: u32) -> DecodeResult<DecodedInstruction> {
        let opcode = Opcode::try_from(inst.opcode())?;
        let mnemonic = match opcode {
            Opcode::Lui => "lui",
            Opcode::Auipc => "auipc",
            _ => return Err(DecodeError::InvalidFormat),
        };
        
        Ok(DecodedInstruction::UType {
            raw: inst,
            opcode,
            mnemonic: mnemonic.to_string(),
            rd: inst.rd(),
            imm: extract_u_immediate(inst),
        })
    }
    
    fn get_mnemonic(&self, _funct3: u8, _funct7: u8) -> DecodeResult<String> {
        Err(DecodeError::InvalidFormat)
    }
}

/// Decoder for J-type instructions (jump operations)
pub struct JTypeDecoder;

impl StandardInstructionDecoder for JTypeDecoder {
    fn format(&self) -> InstructionFormat {
        InstructionFormat::J
    }
    
    fn decode(&self, inst: u32) -> DecodeResult<DecodedInstruction> {
        let opcode = Opcode::try_from(inst.opcode())?;
        if opcode != Opcode::Jal {
            return Err(DecodeError::InvalidFormat);
        }
        
        Ok(DecodedInstruction::JType {
            raw: inst,
            opcode,
            mnemonic: "jal".to_string(),
            rd: inst.rd(),
            imm: extract_j_immediate(inst),
        })
    }
    
    fn get_mnemonic(&self, _funct3: u8, _funct7: u8) -> DecodeResult<String> {
        Ok("jal".to_string())
    }
}

/// Decoder for fence operations (MISC-MEM opcode)
pub struct FenceDecoder;

impl StandardInstructionDecoder for FenceDecoder {
    fn format(&self) -> InstructionFormat {
        InstructionFormat::F
    }
    
    fn decode(&self, inst: u32) -> DecodeResult<DecodedInstruction> {
        let opcode = Opcode::try_from(inst.opcode())?;
        let funct3 = inst.funct3();
        
        let mnemonic = match funct3 {
            0 => "fence",
            1 => "fence.i",
            _ => return Err(DecodeError::InvalidFunct(funct3, 0)),
        };
        
        Ok(DecodedInstruction::FType {
            raw: inst,
            opcode,
            mnemonic: mnemonic.to_string(),
            rd: inst.rd(),
            rs1: inst.rs1(),
            funct3,
            pred: ((inst >> 24) & 0xF) as u8,
            succ: ((inst >> 20) & 0xF) as u8,
        })
    }
    
    fn get_mnemonic(&self, funct3: u8, _funct7: u8) -> DecodeResult<String> {
        match funct3 {
            0 => Ok("fence".to_string()),
            1 => Ok("fence.i".to_string()),
            _ => Err(DecodeError::InvalidFunct(funct3, 0)),
        }
    }
}

/// Decoder for system instructions (SYSTEM opcode)
pub struct SystemDecoder;

impl StandardInstructionDecoder for SystemDecoder {
    fn format(&self) -> InstructionFormat {
        InstructionFormat::I // System uses I-type encoding layout
    }
    
    fn decode(&self, inst: u32) -> DecodeResult<DecodedInstruction> {
        let opcode = Opcode::try_from(inst.opcode())?;
        let funct3 = inst.funct3();
        let rs1 = inst.rs1();
        let rd = inst.rd();
        let csr = (inst >> 20) & 0xFFF;
        
        let mnemonic = match funct3 {
            0 => {
                // ECALL/EBREAK - distinguished by imm field
                match csr {
                    0 => "ecall",
                    1 => "ebreak", 
                    _ => return Err(DecodeError::InvalidFunct(funct3, 0)),
                }
            },
            1 => "csrrw",
            2 => "csrrs", 
            3 => "csrrc",
            5 => "csrrwi",
            6 => "csrrsi",
            7 => "csrrci",
            _ => return Err(DecodeError::InvalidFunct(funct3, 0)),
        };
        
        Ok(DecodedInstruction::System {
            raw: inst,
            opcode,
            mnemonic: mnemonic.to_string(),
            rd,
            rs1,
            funct3,
            csr,
        })
    }
    
    fn get_mnemonic(&self, funct3: u8, _funct7: u8) -> DecodeResult<String> {
        match funct3 {
            0 => Ok("ecall/ebreak".to_string()), // Needs CSR field to distinguish
            1 => Ok("csrrw".to_string()),
            2 => Ok("csrrs".to_string()),
            3 => Ok("csrrc".to_string()),
            5 => Ok("csrrwi".to_string()),
            6 => Ok("csrrsi".to_string()),
            7 => Ok("csrrci".to_string()),
            _ => Err(DecodeError::InvalidFunct(funct3, 0)),
        }
    }
}

/// Decoder for atomic memory operations (A-type instructions)
pub struct ATypeDecoder {
    /// Mapping from (funct3, funct5) to instruction mnemonic
    word_mnemonics: HashMap<(u8, u8), &'static str>,
    doubleword_mnemonics: HashMap<(u8, u8), &'static str>,
}

impl ATypeDecoder {
    pub fn new() -> Self {
        let mut word_mnemonics = HashMap::new();
        let mut doubleword_mnemonics = HashMap::new();
        
        // Word atomic operations (32-bit)
        word_mnemonics.insert((2, 2), "lr.w");      // Load reserved word
        word_mnemonics.insert((2, 3), "sc.w");      // Store conditional word
        word_mnemonics.insert((2, 1), "amoswap.w"); // Atomic swap word
        word_mnemonics.insert((2, 0), "amoadd.w");  // Atomic add word
        word_mnemonics.insert((2, 4), "amoxor.w");  // Atomic XOR word
        word_mnemonics.insert((2, 12), "amoand.w"); // Atomic AND word
        word_mnemonics.insert((2, 8), "amoor.w");   // Atomic OR word
        word_mnemonics.insert((2, 16), "amomin.w"); // Atomic min word
        word_mnemonics.insert((2, 20), "amomax.w"); // Atomic max word
        word_mnemonics.insert((2, 24), "amominu.w"); // Atomic min unsigned word
        word_mnemonics.insert((2, 28), "amomaxu.w"); // Atomic max unsigned word
        
        // Doubleword atomic operations (64-bit)
        doubleword_mnemonics.insert((3, 2), "lr.d");      // Load reserved doubleword
        doubleword_mnemonics.insert((3, 3), "sc.d");      // Store conditional doubleword
        doubleword_mnemonics.insert((3, 1), "amoswap.d"); // Atomic swap doubleword
        doubleword_mnemonics.insert((3, 0), "amoadd.d");  // Atomic add doubleword
        doubleword_mnemonics.insert((3, 4), "amoxor.d");  // Atomic XOR doubleword
        doubleword_mnemonics.insert((3, 12), "amoand.d"); // Atomic AND doubleword
        doubleword_mnemonics.insert((3, 8), "amoor.d");   // Atomic OR doubleword
        doubleword_mnemonics.insert((3, 16), "amomin.d"); // Atomic min doubleword
        doubleword_mnemonics.insert((3, 20), "amomax.d"); // Atomic max doubleword
        doubleword_mnemonics.insert((3, 24), "amominu.d"); // Atomic min unsigned doubleword
        doubleword_mnemonics.insert((3, 28), "amomaxu.d"); // Atomic max unsigned doubleword
        
        Self { word_mnemonics, doubleword_mnemonics }
    }
}

impl StandardInstructionDecoder for ATypeDecoder {
    fn format(&self) -> InstructionFormat {
        InstructionFormat::A
    }
    
    fn decode(&self, inst: u32) -> DecodeResult<DecodedInstruction> {
        let opcode = Opcode::try_from(inst.opcode())?;
        let funct3 = inst.funct3();
        let funct5 = (inst >> 27) & 0x1F; // Extract funct5 from bits [31:27]
        let aq = ((inst >> 26) & 0x1) != 0; // Acquire bit
        let rl = ((inst >> 25) & 0x1) != 0; // Release bit
        
        // Determine if this is word (32-bit) or doubleword (64-bit) operation
        let funct5_u8 = funct5 as u8;
        let mnemonic = match funct3 {
            2 => {
                // Word operations
                self.word_mnemonics.get(&(funct3, funct5_u8))
                    .map(|&s| s.to_string())
                    .ok_or(DecodeError::InvalidFunct(funct3, funct5_u8))?
            },
            3 => {
                // Doubleword operations  
                self.doubleword_mnemonics.get(&(funct3, funct5_u8))
                    .map(|&s| s.to_string())
                    .ok_or(DecodeError::InvalidFunct(funct3, funct5_u8))?
            },
            _ => return Err(DecodeError::InvalidFunct(funct3, funct5_u8)),
        };
        
        Ok(DecodedInstruction::AType {
            raw: inst,
            opcode,
            mnemonic,
            rd: inst.rd(),
            rs1: inst.rs1(),
            rs2: inst.rs2(),
            funct3,
            funct5: funct5_u8,
            aq,
            rl,
        })
    }
    
    fn get_mnemonic(&self, funct3: u8, funct5: u8) -> DecodeResult<String> {
        // Try word operations first
        if let Some(&mnemonic) = self.word_mnemonics.get(&(funct3, funct5)) {
            return Ok(mnemonic.to_string());
        }
        
        // Try doubleword operations
        if let Some(&mnemonic) = self.doubleword_mnemonics.get(&(funct3, funct5)) {
            return Ok(mnemonic.to_string());
        }
        
        Err(DecodeError::InvalidFunct(funct3, funct5))
    }
}
