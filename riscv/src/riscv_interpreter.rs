//! Parses a 32-bits RISC-V instruction

use crate::{RiscvInstruction, Rvd, RvdOperation};

/// Convert 32-bits data chunk that contains a signed integer of a specified size in bits to a
/// signed integer of 32 bits
fn signext(v: u32, size: u32) -> i32 {
    let sign_bit: u32 = 1u32 << (size - 1);
    let max_value: u32 = 1u32 << size;
    if (sign_bit & v) != 0 {
        v as i32 - max_value as i32
    } else {
        v as i32
    }
}

/// Gets the RUSTC instruction in text and tree level, based on the RVD operation and 2 tree
/// branches indexes
fn getinst(op: &RvdOperation, i1: u32, i2: u32) -> (String, i32) {
    if !op.s.is_empty() {
        return (op.s.clone(), 0);
    }
    if !op.map.contains_key(&i1) {
        return (String::new(), -1);
    }
    if !op.map[&i1].s.is_empty() {
        return (op.map[&i1].s.clone(), 1);
    }
    if !op.map[&i1].map.contains_key(&i2) {
        return (String::new(), -1);
    }
    if !op.map[&i1].map[&i2].s.is_empty() {
        return (op.map[&i1].map[&i2].s.clone(), 2);
    }
    (String::new(), -1)
}

/// Interprets a buffer of 32-bits RICSV instructions into a vector of decoded RISCV instructions
/// split by field
pub fn riscv_interpreter(code: &[u32]) -> Vec<RiscvInstruction> {
    let mut insts = Vec::<RiscvInstruction>::new();

    // Build an RVD data tree
    let mut rvd = Rvd::new();
    rvd.init();

    // For every 32-bit instruction in the input code buffer
    let code_len = code.len();
    for (s, inst_ref) in code.iter().enumerate().take(code_len) {
        //println!("riscv_interpreter() s={}", s);

        // Get the RISCV instruction
        let inst = *inst_ref;

        // Ignore instructions that are zero
        if inst == 0 {
            //println!("riscv_interpreter() found inst=0 at position s={}", s);
            continue;
        }

        // Extract the opcode from the lower 7 bits of the RICSV instruction
        let opcode = inst & 0x7F;

        // Get the RVD info data for this opcode
        if !rvd.opcodes.contains_key(&opcode) {
            panic!("Invalid opcode={opcode}=0x{opcode:x} s={s}");
        }
        let inf = &rvd.opcodes[&opcode];

        // Create a RISCV instruction instance to be filled with data from the instruction and from
        // the RVD info data
        // Copy the original RISCV 32-bit instruction
        // Copy the instruction type
        let mut i = RiscvInstruction { rvinst: inst, t: inf.t.clone(), ..Default::default() };

        // Decode the rest of instruction fields based on the instruction type

        //  31 30 ... 21 20 19 ... 15 14 13 12 11 ... 07 06 05 04 03 02 01 00
        // |  imm[11:0]    |  rs1    | funct3 |   rd    |       opcode       | I-type
        if i.t == *"I" {
            i.funct3 = (inst & 0x7000) >> 12;
            let funct7 = (inst & 0xFC000000) >> 26;
            i.rd = (inst & 0xF80) >> 7;
            i.rs1 = (inst & 0xF8000) >> 15;
            i.imm = signext((inst & 0xFFF00000) >> 20, 12);
            let l: i32;
            (i.inst, l) = getinst(&inf.op, i.funct3, funct7);
            assert!(!i.inst.is_empty());
            if l == 2 {
                i.imm &= 0x3F;
                i.funct7 = funct7;
            }
        }
        //  31 30 ... 26 25 24 ... 20 19 ... 15 14 13 12 11 ... 07 06 05 04 03 02 01 00
        // |   funct7      |  rs2    |  rs1    | funct3 |   rd    |       opcode       | R-type
        else if i.t == *"R" {
            i.funct3 = (inst & 0x7000) >> 12;
            i.rd = (inst & 0xF80) >> 7;
            i.rs1 = (inst & 0xF8000) >> 15;
            i.rs2 = (inst & 0x1F00000) >> 20;
            i.funct7 = (inst & 0xFE000000) >> 25;
            (i.inst, _) = getinst(&inf.op, i.funct3, i.funct7);
            assert!(!i.inst.is_empty());
        }
        //  31 30 ... 26 25 24 ... 20 19 ... 15 14 13 12 11 10 09 08 07 06 05 04 03 02 01 00
        // |  imm[11:5]    |  rs2    |   rs1   | funct3 |   imm[4:0]   |       opcode       | S-type
        else if i.t == *"S" {
            i.funct3 = (inst & 0x7000) >> 12;
            let imm4_0 = (inst & 0xF80) >> 7;
            i.rs1 = (inst & 0xF8000) >> 15;
            i.rs2 = (inst & 0x1F00000) >> 20;
            let imm11_5 = (inst & 0xFE000000) >> 25;
            i.imm = signext((imm11_5 << 5) | imm4_0, 12);
            (i.inst, _) = getinst(&inf.op, i.funct3, 0);
            assert!(!i.inst.is_empty());
        }
        //  31 30 29 28 27 26 25 24...20 19...15 14 13 12 11 10 09 08 07 06 05 04 03 02 01 00
        // |12|    imm[10:5]    |  rs2  | rs1   | funct3 |imm[4:1]   |11|       opcode       | B-type
        else if i.t == *"B" {
            i.funct3 = (inst & 0x7000) >> 12;
            let imm11 = (inst & 0x080) >> 7;
            let imm4_1 = (inst & 0xF00) >> 8;
            i.rs1 = (inst & 0xF8000) >> 15;
            i.rs2 = (inst & 0x1F00000) >> 20;
            let imm10_5 = (inst & 0x7E000000) >> 25;
            let imm12 = (inst & 0x80000000) >> 31;
            i.imm = signext((imm12 << 12) | (imm11 << 11) | (imm10_5 << 5) | (imm4_1 << 1), 13);
            (i.inst, _) = getinst(&inf.op, i.funct3, 0);
            assert!(!i.inst.is_empty());
        }
        //  31 30 ... 13 12 11 10 09 08 07 06 05 04 03 02 01 00
        // |  imm[31:12]   |      rd      |        opcode      | U-type
        else if i.t == *"U" {
            i.rd = (inst & 0xF80) >> 7;
            i.imm = (((inst & 0xFFFFF000) >> 12) << 12) as i32;
            (i.inst, _) = getinst(&inf.op, 0, 0);
            assert!(!i.inst.is_empty());
        }
        //  31 30 29...22 21 20 19 18 ... 13 12 11 10 09 08 07 06 05 04 03 02 01 00
        // |20|  imm[10:1]  |11|  imm[19:12]   |      rd      |       opcode       | J-type
        else if i.t == *"J" {
            i.rd = (inst & 0xF80) >> 7;
            let imm20 = (inst & 0x80000000) >> 31;
            let imm10_1 = (inst & 0x7FE00000) >> 21;
            let imm11j = (inst & 0x100000) >> 20;
            let imm19_12 = (inst & 0xFF000) >> 12;
            i.imm = signext((imm20 << 20) | (imm19_12 << 12) | (imm11j << 11) | (imm10_1 << 1), 21);
            (i.inst, _) = getinst(&inf.op, 0, 0);
            assert!(!i.inst.is_empty());
        } else if i.t == *"A" {
            i.funct3 = (inst & 0x7000) >> 12;
            i.rd = (inst & 0xF80) >> 7;
            i.rs1 = (inst & 0xF8000) >> 15;
            i.rs2 = (inst & 0x1F00000) >> 20;
            i.funct5 = (inst & 0xF8000000) >> 27;
            i.aq = (inst & 0x4000000) >> 26;
            i.rl = (inst & 0x2000000) >> 24;
            (i.inst, _) = getinst(&inf.op, i.funct3, i.funct5);
            assert!(!i.inst.is_empty());
        } else if i.t == *"C" {
            i.funct3 = (inst & 0x7000) >> 12;
            if i.funct3 == 0 {
                if inst == 0x00000073 {
                    i.inst = "ecall".to_string();
                } else if inst == 0x00100073 {
                    i.inst = "ebreak".to_string();
                } else {
                    i.inst = "ecall".to_string();
                    // TODO check what means this extra bits in ECALL
                    // throw new Error(`Invalid opcode: ${opcode} at line ${s}`);
                }
            } else {
                i.rd = (inst & 0xF80) >> 7;
                if (i.funct3 & 0x4) != 0 {
                    i.imme = (inst & 0xF8000) >> 15;
                } else {
                    i.rs1 = (inst & 0xF8000) >> 15;
                }
                i.csr = (inst & 0xFFF00000) >> 20;
                (i.inst, _) = getinst(&inf.op, i.funct3, 0);
                assert!(!i.inst.is_empty());
            }
        } else if i.t == *"F" {
            i.funct3 = (inst & 0x7000) >> 12;
            if i.funct3 == 0 {
                if (inst & 0xF00F8F80) != 0 {
                    panic!("Invalid opcode={opcode} at line s={s}");
                }
                i.pred = (inst & 0x0F000000) >> 24;
                i.succ = (inst & 0x00F00000) >> 20;
                i.inst = "fence".to_string();
            } else if i.funct3 == 1 {
                if (inst & 0xFFFF8F80) != 0 {
                    panic!("Invalid opcode={opcode} at line s={s}");
                }
                i.inst = "fence.i".to_string();
            } else {
                panic!("Invalid opcode={opcode} at line s={s}");
            }
        } else {
            panic!("Invalid i.t={} at line s={}", i.t, s);
        }
        insts.push(i);
    }
    insts
}

/// Decodes a compressed (16-bit) RISC-V instruction into its constituent fields
fn decode_compressed_instruction(inst: u16, addr: u64) -> RiscvInstruction {
    let mut i = RiscvInstruction {
        rvinst: inst as u32,
        is_compressed: true,
        addr,
        c_op: (inst & 0x3) as u32,
        ..Default::default()
    };

    // Extract common fields
    i.funct3 = ((inst >> 13) & 0x7) as u32;
    
    match i.c_op {
        0b00 => {
            // C0 Quadrant
            match i.funct3 {
                0b000 => {
                    // C.ADDI4SPN
                    if inst == 0 {
                        i.inst = "illegal".to_string();
                        i.t = "C".to_string();
                    } else {
                        let nzimm = (((inst >> 7) & 0x30) | ((inst >> 1) & 0x3c0) | 
                                    ((inst >> 4) & 0x4) | ((inst >> 2) & 0x8)) as i32;
                        i.rd = ((inst >> 2) & 0x7) as u32 + 8; // Map to x8-x15
                        i.rs1 = 2; // sp
                        i.imm = nzimm;
                        i.inst = "addi".to_string();
                        i.t = "I".to_string();
                    }
                },
                0b010 => {
                    // C.LW
                    let offset = (((inst >> 7) & 0x38) | ((inst >> 4) & 0x4)) as i32;
                    i.rd = ((inst >> 2) & 0x7) as u32 + 8;
                    i.rs1 = ((inst >> 7) & 0x7) as u32 + 8;
                    i.imm = offset;
                    i.inst = "lw".to_string();
                    i.t = "I".to_string();
                },
                0b110 => {
                    // C.SW
                    let offset = (((inst >> 7) & 0x38) | ((inst >> 4) & 0x4)) as i32;
                    i.rs1 = ((inst >> 7) & 0x7) as u32 + 8;
                    i.rs2 = ((inst >> 2) & 0x7) as u32 + 8;
                    i.imm = offset;
                    i.inst = "sw".to_string();
                    i.t = "S".to_string();
                },
                _ => {
                    i.inst = "illegal".to_string();
                    i.t = "C".to_string();
                }
            }
        },
        0b01 => {
            // C1 Quadrant
            match i.funct3 {
                0b000 => {
                    // C.ADDI or C.NOP
                    i.rd = ((inst >> 7) & 0x1f) as u32;
                    let imm = (((inst >> 7) & 0x20) as i32) >> 5; // Sign extend bit 5
                    let imm = imm | (((inst >> 2) & 0x1f) as i32);
                    if i.rd == 0 && imm == 0 {
                        i.inst = "nop".to_string();
                    } else {
                        i.inst = "addi".to_string();
                        i.rs1 = i.rd;
                        i.imm = if (inst & 0x1000) != 0 { imm | !0x1f } else { imm }; // Sign extend
                    }
                    i.t = "I".to_string();
                },
                0b001 => {
                    // C.JAL (RV32 only) / C.ADDIW (RV64)
                    let offset = sign_extend_c_j_imm(inst);
                    i.rd = 1; // x1 (ra)
                    i.imm = offset;
                    i.inst = "jal".to_string();
                    i.t = "J".to_string();
                },
                0b010 => {
                    // C.LI
                    i.rd = ((inst >> 7) & 0x1f) as u32;
                    let imm = (((inst >> 7) & 0x20) as i32) >> 5; // Sign extend bit 5
                    i.imm = if (inst & 0x1000) != 0 { 
                        imm | (((inst >> 2) & 0x1f) as i32) | !0x1f 
                    } else { 
                        imm | (((inst >> 2) & 0x1f) as i32) 
                    };
                    i.inst = "addi".to_string();
                    i.rs1 = 0; // x0
                    i.t = "I".to_string();
                },
                0b011 => {
                    let rd = ((inst >> 7) & 0x1f) as u32;
                    if rd == 2 {
                        // C.ADDI16SP
                        let nzimm = sign_extend_c_addi16sp_imm(inst);
                        i.rd = 2;
                        i.rs1 = 2;
                        i.imm = nzimm;
                        i.inst = "addi".to_string();
                        i.t = "I".to_string();
                    } else if rd != 0 {
                        // C.LUI
                        let nzimm = sign_extend_c_lui_imm(inst);
                        i.rd = rd;
                        i.imm = nzimm;
                        i.inst = "lui".to_string();
                        i.t = "U".to_string();
                    } else {
                        i.inst = "illegal".to_string();
                        i.t = "C".to_string();
                    }
                },
                0b100 => {
                    // C.SRLI, C.SRAI, C.ANDI, C.SUB, C.XOR, C.OR, C.AND
                    let funct2 = ((inst >> 10) & 0x3) as u32;
                    i.rd = ((inst >> 7) & 0x7) as u32 + 8;
                    i.rs1 = i.rd;
                    
                    match funct2 {
                        0b00 => {
                            // C.SRLI
                            i.imm = ((inst >> 2) & 0x1f) as i32;
                            i.inst = "srli".to_string();
                            i.t = "I".to_string();
                        },
                        0b01 => {
                            // C.SRAI
                            i.imm = ((inst >> 2) & 0x1f) as i32;
                            i.inst = "srai".to_string();
                            i.t = "I".to_string();
                        },
                        0b10 => {
                            // C.ANDI
                            let imm = (((inst >> 7) & 0x20) as i32) >> 5; // Sign extend bit 5
                            i.imm = if (inst & 0x1000) != 0 { 
                                imm | (((inst >> 2) & 0x1f) as i32) | !0x1f 
                            } else { 
                                imm | (((inst >> 2) & 0x1f) as i32) 
                            };
                            i.inst = "andi".to_string();
                            i.t = "I".to_string();
                        },
                        0b11 => {
                            // C.SUB, C.XOR, C.OR, C.AND
                            let funct6 = ((inst >> 12) & 0x1) as u32;
                            let funct2_low = ((inst >> 5) & 0x3) as u32;
                            i.rs2 = ((inst >> 2) & 0x7) as u32 + 8;
                            
                            if funct6 == 0 {
                                match funct2_low {
                                    0b00 => i.inst = "sub".to_string(),
                                    0b01 => i.inst = "xor".to_string(),
                                    0b10 => i.inst = "or".to_string(),
                                    0b11 => i.inst = "and".to_string(),
                                    _ => i.inst = "illegal".to_string(),
                                }
                            } else {
                                i.inst = "illegal".to_string();
                            }
                            i.t = "R".to_string();
                        },
                        _ => {
                            i.inst = "illegal".to_string();
                            i.t = "C".to_string();
                        }
                    }
                },
                0b101 => {
                    // C.J
                    let offset = sign_extend_c_j_imm(inst);
                    i.rd = 0; // x0
                    i.imm = offset;
                    i.inst = "jal".to_string();
                    i.t = "J".to_string();
                },
                0b110 => {
                    // C.BEQZ
                    let offset = sign_extend_c_b_imm(inst);
                    i.rs1 = ((inst >> 7) & 0x7) as u32 + 8;
                    i.rs2 = 0; // x0
                    i.imm = offset;
                    i.inst = "beq".to_string();
                    i.t = "B".to_string();
                },
                0b111 => {
                    // C.BNEZ
                    let offset = sign_extend_c_b_imm(inst);
                    i.rs1 = ((inst >> 7) & 0x7) as u32 + 8;
                    i.rs2 = 0; // x0
                    i.imm = offset;
                    i.inst = "bne".to_string();
                    i.t = "B".to_string();
                },
                _ => {
                    i.inst = "illegal".to_string();
                    i.t = "C".to_string();
                }
            }
        },
        0b10 => {
            // C2 Quadrant
            match i.funct3 {
                0b000 => {
                    // C.SLLI
                    i.rd = ((inst >> 7) & 0x1f) as u32;
                    i.rs1 = i.rd;
                    i.imm = ((inst >> 2) & 0x1f) as i32;
                    i.inst = "slli".to_string();
                    i.t = "I".to_string();
                },
                0b010 => {
                    // C.LWSP
                    i.rd = ((inst >> 7) & 0x1f) as u32;
                    let offset = (((inst >> 4) & 0x4) | ((inst >> 7) & 0x20) | ((inst >> 2) & 0x1c)) as i32;
                    i.rs1 = 2; // sp
                    i.imm = offset;
                    i.inst = "lw".to_string();
                    i.t = "I".to_string();
                },
                0b100 => {
                    let funct4 = ((inst >> 12) & 0x1) as u32;
                    let rs1 = ((inst >> 7) & 0x1f) as u32;
                    let rs2 = ((inst >> 2) & 0x1f) as u32;
                    
                    if funct4 == 0 {
                        if rs2 == 0 {
                            if rs1 == 0 {
                                i.inst = "illegal".to_string();
                                i.t = "C".to_string();
                            } else {
                                // C.JR
                                i.rs1 = rs1;
                                i.rd = 0;
                                i.imm = 0;
                                i.inst = "jalr".to_string();
                                i.t = "I".to_string();
                            }
                        } else {
                            // C.MV
                            i.rd = rs1;
                            i.rs1 = 0; // x0
                            i.rs2 = rs2;
                            i.inst = "add".to_string();
                            i.t = "R".to_string();
                        }
                    } else {
                        if rs2 == 0 {
                            if rs1 == 0 {
                                // C.EBREAK
                                i.inst = "ebreak".to_string();
                                i.t = "C".to_string();
                            } else {
                                // C.JALR
                                i.rs1 = rs1;
                                i.rd = 1; // x1
                                i.imm = 0;
                                i.inst = "jalr".to_string();
                                i.t = "I".to_string();
                            }
                        } else {
                            // C.ADD
                            i.rd = rs1;
                            i.rs1 = rs1;
                            i.rs2 = rs2;
                            i.inst = "add".to_string();
                            i.t = "R".to_string();
                        }
                    }
                },
                0b110 => {
                    // C.SWSP
                    let offset = (((inst >> 9) & 0x3c) | ((inst >> 7) & 0x40)) as i32;
                    i.rs1 = 2; // sp
                    i.rs2 = ((inst >> 2) & 0x1f) as u32;
                    i.imm = offset;
                    i.inst = "sw".to_string();
                    i.t = "S".to_string();
                },
                _ => {
                    i.inst = "illegal".to_string();
                    i.t = "C".to_string();
                }
            }
        },
        _ => {
            i.inst = "illegal".to_string();
            i.t = "C".to_string();
        }
    }

    i
}

// Helper functions for compressed instruction immediate decoding
fn sign_extend_c_j_imm(inst: u16) -> i32 {
    let imm = (((inst >> 3) & 0x8) | ((inst >> 7) & 0x10) | ((inst >> 1) & 0x300) |
               ((inst >> 4) & 0x400) | ((inst << 2) & 0x40) | ((inst >> 1) & 0x20) |
               ((inst << 3) & 0x80) | ((inst >> 1) & 0x4) | ((inst << 1) & 0x200)) as i32;
    
    // Sign extend from bit 11
    if (inst & 0x1000) != 0 {
        imm | !0x7ff
    } else {
        imm
    }
}

fn sign_extend_c_b_imm(inst: u16) -> i32 {
    let imm = (((inst >> 4) & 0x100) | ((inst >> 7) & 0x18) | ((inst << 1) & 0x40) |
               ((inst >> 1) & 0x20) | ((inst << 3) & 0x80) | ((inst >> 2) & 0x4) |
               ((inst << 1) & 0x200)) as i32;
    
    // Sign extend from bit 8
    if (inst & 0x1000) != 0 {
        imm | !0x1ff
    } else {
        imm
    }
}

fn sign_extend_c_addi16sp_imm(inst: u16) -> i32 {
    let imm = (((inst >> 3) & 0x200) | ((inst >> 2) & 0x10) | ((inst << 1) & 0x40) |
               ((inst << 4) & 0x180) | ((inst << 3) & 0x20)) as i32;
    
    // Sign extend from bit 9
    if (inst & 0x1000) != 0 {
        imm | !0x3ff
    } else {
        imm
    }
}

fn sign_extend_c_lui_imm(inst: u16) -> i32 {
    let imm = (((inst >> 7) & 0x20) | ((inst >> 2) & 0x1f)) << 12;
    
    // Sign extend from bit 17
    if (inst & 0x1000) != 0 {
        (imm as i32) | !0x1ffff
    } else {
        imm as i32
    }
}

/// Interprets a buffer of mixed 16/32-bit RISC-V instructions into a vector of decoded RISCV instructions
pub fn riscv_interpreter_mixed(instruction_words: &[crate::RiscvInstructionWord]) -> Vec<RiscvInstruction> {
    let mut insts = Vec::<RiscvInstruction>::new();

    // Build an RVD data tree for 32-bit instructions
    let mut rvd = Rvd::new();
    rvd.init();

    for inst_word in instruction_words {
        if inst_word.is_compressed {
            // Handle compressed instruction
            let compressed_inst = decode_compressed_instruction(inst_word.instruction as u16, inst_word.addr);
            insts.push(compressed_inst);
        } else {
            // Handle uncompressed instruction using existing logic
            let inst = inst_word.instruction;
            
            // Ignore instructions that are zero
            if inst == 0 {
                continue;
            }

            // Extract the opcode from the lower 7 bits
            let opcode = inst & 0x7F;

            // Get the RVD info data for this opcode
            if !rvd.opcodes.contains_key(&opcode) {
                panic!("Invalid opcode={opcode}=0x{opcode:x} addr=0x{:x}", inst_word.addr);
            }
            let inf = &rvd.opcodes[&opcode];

            // Create a RISCV instruction instance
            let mut i = RiscvInstruction { 
                rvinst: inst, 
                t: inf.t.clone(), 
                is_compressed: false,
                addr: inst_word.addr,
                ..Default::default() 
            };

            // Decode using existing logic (same as original function)
            if i.t == *"I" {
                i.funct3 = (inst & 0x7000) >> 12;
                let funct7 = (inst & 0xFC000000) >> 26;
                i.rd = (inst & 0xF80) >> 7;
                i.rs1 = (inst & 0xF8000) >> 15;
                i.imm = signext((inst & 0xFFF00000) >> 20, 12);
                let l: i32;
                (i.inst, l) = getinst(&inf.op, i.funct3, funct7);
                assert!(!i.inst.is_empty());
                if l == 2 {
                    i.imm &= 0x3F;
                    i.funct7 = funct7;
                }
            } else if i.t == *"R" {
                i.funct3 = (inst & 0x7000) >> 12;
                i.rd = (inst & 0xF80) >> 7;
                i.rs1 = (inst & 0xF8000) >> 15;
                i.rs2 = (inst & 0x1F00000) >> 20;
                i.funct7 = (inst & 0xFE000000) >> 25;
                (i.inst, _) = getinst(&inf.op, i.funct3, i.funct7);
                assert!(!i.inst.is_empty());
            } else if i.t == *"S" {
                i.funct3 = (inst & 0x7000) >> 12;
                let imm4_0 = (inst & 0xF80) >> 7;
                i.rs1 = (inst & 0xF8000) >> 15;
                i.rs2 = (inst & 0x1F00000) >> 20;
                let imm11_5 = (inst & 0xFE000000) >> 25;
                i.imm = signext((imm11_5 << 5) | imm4_0, 12);
                (i.inst, _) = getinst(&inf.op, i.funct3, 0);
                assert!(!i.inst.is_empty());
            } else if i.t == *"B" {
                i.funct3 = (inst & 0x7000) >> 12;
                let imm11 = (inst & 0x080) >> 7;
                let imm4_1 = (inst & 0xF00) >> 8;
                i.rs1 = (inst & 0xF8000) >> 15;
                i.rs2 = (inst & 0x1F00000) >> 20;
                let imm10_5 = (inst & 0x7E000000) >> 25;
                let imm12 = (inst & 0x80000000) >> 31;
                i.imm = signext((imm12 << 12) | (imm11 << 11) | (imm10_5 << 5) | (imm4_1 << 1), 13);
                (i.inst, _) = getinst(&inf.op, i.funct3, 0);
                assert!(!i.inst.is_empty());
            } else if i.t == *"U" {
                i.rd = (inst & 0xF80) >> 7;
                i.imm = (((inst & 0xFFFFF000) >> 12) << 12) as i32;
                (i.inst, _) = getinst(&inf.op, 0, 0);
                assert!(!i.inst.is_empty());
            } else if i.t == *"J" {
                i.rd = (inst & 0xF80) >> 7;
                let imm20 = (inst & 0x80000000) >> 31;
                let imm10_1 = (inst & 0x7FE00000) >> 21;
                let imm11j = (inst & 0x100000) >> 20;
                let imm19_12 = (inst & 0xFF000) >> 12;
                i.imm = signext((imm20 << 20) | (imm19_12 << 12) | (imm11j << 11) | (imm10_1 << 1), 21);
                (i.inst, _) = getinst(&inf.op, 0, 0);
                assert!(!i.inst.is_empty());
            } else if i.t == *"A" {
                i.funct3 = (inst & 0x7000) >> 12;
                i.rd = (inst & 0xF80) >> 7;
                i.rs1 = (inst & 0xF8000) >> 15;
                i.rs2 = (inst & 0x1F00000) >> 20;
                i.funct5 = (inst & 0xF8000000) >> 27;
                i.aq = (inst & 0x4000000) >> 26;
                i.rl = (inst & 0x2000000) >> 24;
                (i.inst, _) = getinst(&inf.op, i.funct3, i.funct5);
                assert!(!i.inst.is_empty());
            } else if i.t == *"C" {
                i.funct3 = (inst & 0x7000) >> 12;
                if i.funct3 == 0 {
                    if inst == 0x00000073 {
                        i.inst = "ecall".to_string();
                    } else if inst == 0x00100073 {
                        i.inst = "ebreak".to_string();
                    } else {
                        i.inst = "ecall".to_string();
                    }
                } else {
                    i.rd = (inst & 0xF80) >> 7;
                    if (i.funct3 & 0x4) != 0 {
                        i.imme = (inst & 0xF8000) >> 15;
                    } else {
                        i.rs1 = (inst & 0xF8000) >> 15;
                    }
                    i.csr = (inst & 0xFFF00000) >> 20;
                    (i.inst, _) = getinst(&inf.op, i.funct3, 0);
                    assert!(!i.inst.is_empty());
                }
            } else if i.t == *"F" {
                i.funct3 = (inst & 0x7000) >> 12;
                if i.funct3 == 0 {
                    if (inst & 0xF00F8F80) != 0 {
                        panic!("Invalid opcode={opcode} at addr=0x{:x}", inst_word.addr);
                    }
                    i.pred = (inst & 0x0F000000) >> 24;
                    i.succ = (inst & 0x00F00000) >> 20;
                    i.inst = "fence".to_string();
                } else if i.funct3 == 1 {
                    if (inst & 0xFFFF8F80) != 0 {
                        panic!("Invalid opcode={opcode} at addr=0x{:x}", inst_word.addr);
                    }
                    i.inst = "fence.i".to_string();
                } else {
                    panic!("Invalid opcode={opcode} at addr=0x{:x}", inst_word.addr);
                }
            } else {
                panic!("Invalid i.t={} at addr=0x{:x}", i.t, inst_word.addr);
            }
            
            insts.push(i);
        }
    }
    
    insts
}
