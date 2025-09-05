//! RISC-V compressed (16-bit) instruction decoders
//!
//! Implements decoding for the RVC (compressed) extension which provides
//! 16-bit encodings for common RISC-V instructions to improve code density.

use crate::decoder::{CompressedInstructionDecoder, XLen};
use crate::instruction::{CompressedFormat, DecodeError, DecodeResult, DecodedInstruction, Opcode};

/// Utility function to convert compressed register index (3-bit) to full register index
/// Maps compressed register rs1'/rs2'/rd' (3 bits) to x8-x15 range
fn convert_compressed_reg(reg: u8) -> u8 {
    match reg & 0x7 {
        0 => 8,  // x8
        1 => 9,  // x9
        2 => 10, // x10
        3 => 11, // x11
        4 => 12, // x12
        5 => 13, // x13
        6 => 14, // x14
        7 => 15, // x15
        _ => unreachable!("reg & 0x7 can only be 0-7"),
    }
}

/// Decoder for Quadrant 0 compressed instructions (bits [1:0] = 00)
pub struct Quadrant0Decoder;

impl CompressedInstructionDecoder for Quadrant0Decoder {
    fn quadrant(&self) -> u8 {
        0
    }

    fn decode(&self, inst: u16) -> DecodeResult<DecodedInstruction> {
        if inst == 0x0000 {
            // 0x0000 is reserved/illegal in compressed instruction space
            return Ok(DecodedInstruction::compressed_illegal());
        }

        let funct3 = (inst >> 13) & 0x7;

        match funct3 {
            0x0 => {
                // c.addi4spn → addi rd', x2, nzuimm[9:2]
                let rd_prime = (inst >> 2) & 0x7;
                let nzuimm = extract_ciw_immediate(inst);

                if nzuimm == 0 {
                    return Err(DecodeError::Reserved);
                }

                Ok(DecodedInstruction::Compressed {
                    raw: inst,
                    compressed_format: CompressedFormat::CIW,
                    compressed_mnemonic: "c.addi4spn".to_string(),
                    expanded: Box::new(DecodedInstruction::IType {
                        raw: expand_ciw_to_addi(inst),
                        opcode: Opcode::OpImm,
                        mnemonic: "addi".to_string(),
                        rd: convert_compressed_reg(rd_prime as u8),
                        rs1: 2, // x2 (stack pointer)
                        imm: nzuimm,
                        funct3: 0,
                        funct7: 0,
                    }),
                })
            }
            0x2 => {
                // c.lw → lw rd', offset(rs1')
                let rd_prime = (inst >> 2) & 0x7;
                let rs1_prime = (inst >> 7) & 0x7;
                let offset = extract_cl_lw_immediate(inst);

                Ok(DecodedInstruction::Compressed {
                    raw: inst,
                    compressed_format: CompressedFormat::CL,
                    compressed_mnemonic: "c.lw".to_string(),
                    expanded: Box::new(DecodedInstruction::IType {
                        raw: expand_cl_to_lw(inst),
                        opcode: Opcode::Load,
                        mnemonic: "lw".to_string(),
                        rd: convert_compressed_reg(rd_prime as u8),
                        rs1: convert_compressed_reg(rs1_prime as u8),
                        imm: offset,
                        funct3: 2, // lw funct3
                        funct7: 0,
                    }),
                })
            }
            0x3 => {
                // c.ld → ld rd', offset(rs1') (RV64/128)
                let rd_prime = (inst >> 2) & 0x7;
                let rs1_prime = (inst >> 7) & 0x7;
                let offset = extract_cl_ld_immediate(inst);

                Ok(DecodedInstruction::Compressed {
                    raw: inst,
                    compressed_format: CompressedFormat::CL,
                    compressed_mnemonic: "c.ld".to_string(),
                    expanded: Box::new(DecodedInstruction::IType {
                        raw: expand_cl_to_ld(inst),
                        opcode: Opcode::Load,
                        mnemonic: "ld".to_string(),
                        rd: convert_compressed_reg(rd_prime as u8),
                        rs1: convert_compressed_reg(rs1_prime as u8),
                        imm: offset,
                        funct3: 3, // ld funct3
                        funct7: 0,
                    }),
                })
            }
            0x6 => {
                // c.sw → sw rs2', offset(rs1')
                let rs2_prime = (inst >> 2) & 0x7;
                let rs1_prime = (inst >> 7) & 0x7;
                let offset = extract_cs_sw_immediate(inst);

                Ok(DecodedInstruction::Compressed {
                    raw: inst,
                    compressed_format: CompressedFormat::CS,
                    compressed_mnemonic: "c.sw".to_string(),
                    expanded: Box::new(DecodedInstruction::SType {
                        raw: expand_cs_to_sw(inst),
                        opcode: Opcode::Store,
                        mnemonic: "sw".to_string(),
                        rs1: convert_compressed_reg(rs1_prime as u8),
                        rs2: convert_compressed_reg(rs2_prime as u8),
                        imm: offset,
                        funct3: 2, // sw funct3
                    }),
                })
            }
            0x7 => {
                // c.sd → sd rs2', offset(rs1') (RV64/128)
                let rs2_prime = (inst >> 2) & 0x7;
                let rs1_prime = (inst >> 7) & 0x7;
                let offset = extract_cs_sd_immediate(inst);

                Ok(DecodedInstruction::Compressed {
                    raw: inst,
                    compressed_format: CompressedFormat::CS,
                    compressed_mnemonic: "c.sd".to_string(),
                    expanded: Box::new(DecodedInstruction::SType {
                        raw: expand_cs_to_sd(inst),
                        opcode: Opcode::Store,
                        mnemonic: "sd".to_string(),
                        rs1: convert_compressed_reg(rs1_prime as u8),
                        rs2: convert_compressed_reg(rs2_prime as u8),
                        imm: offset,
                        funct3: 3, // sd funct3
                    }),
                })
            }
            0x1 | 0x4 | 0x5 => {
                // Reserved or floating point (not supported)
                Err(DecodeError::Reserved)
            }
            _ => Err(DecodeError::InvalidFunct(funct3 as u8, 0)),
        }
    }
}

/// Decoder for Quadrant 1 compressed instructions (bits [1:0] = 01)
pub struct Quadrant1Decoder {
    xlen: XLen,
}

impl Quadrant1Decoder {
    pub fn new(xlen: XLen) -> Self {
        Self { xlen }
    }
}

impl CompressedInstructionDecoder for Quadrant1Decoder {
    fn quadrant(&self) -> u8 {
        1
    }

    fn decode(&self, inst: u16) -> DecodeResult<DecodedInstruction> {
        let funct3 = (inst >> 13) & 0x7;

        match funct3 {
            0x0 => {
                // c.nop or c.addi
                let rd = (inst >> 7) & 0x1F;

                if rd == 0 {
                    // c.nop → addi x0, x0, 0
                    Ok(DecodedInstruction::Compressed {
                        raw: inst,
                        compressed_format: CompressedFormat::CI,
                        compressed_mnemonic: "c.nop".to_string(),
                        expanded: Box::new(DecodedInstruction::nop()),
                    })
                } else {
                    // c.addi → addi rd, rd, imm
                    let imm = extract_ci_addi_immediate(inst);

                    Ok(DecodedInstruction::Compressed {
                        raw: inst,
                        compressed_format: CompressedFormat::CI,
                        compressed_mnemonic: "c.addi".to_string(),
                        expanded: Box::new(DecodedInstruction::IType {
                            raw: expand_ci_to_addi(inst),
                            opcode: Opcode::OpImm,
                            mnemonic: "addi".to_string(),
                            rd: rd as u8,
                            rs1: rd as u8, // c.addi uses same reg for src/dest
                            imm,
                            funct3: 0,
                            funct7: 0,
                        }),
                    })
                }
            }
            0x1 => {
                // c.addiw → addiw rd, rd, imm (RV64/128)
                let rd = (inst >> 7) & 0x1F;
                let imm = extract_ci_addi_immediate(inst);

                if rd == 0 {
                    return Err(DecodeError::Reserved);
                }

                Ok(DecodedInstruction::Compressed {
                    raw: inst,
                    compressed_format: CompressedFormat::CI,
                    compressed_mnemonic: "c.addiw".to_string(),
                    expanded: Box::new(DecodedInstruction::IType {
                        raw: expand_ci_to_addiw(inst),
                        opcode: Opcode::OpImm32,
                        mnemonic: "addiw".to_string(),
                        rd: rd as u8,
                        rs1: rd as u8,
                        imm,
                        funct3: 0,
                        funct7: 0,
                    }),
                })
            }
            0x2 => {
                // c.li → addi rd, x0, imm
                let rd = (inst >> 7) & 0x1F;
                let imm = extract_ci_addi_immediate(inst);

                Ok(DecodedInstruction::Compressed {
                    raw: inst,
                    compressed_format: CompressedFormat::CI,
                    compressed_mnemonic: "c.li".to_string(),
                    expanded: Box::new(DecodedInstruction::IType {
                        raw: expand_ci_to_li(inst),
                        opcode: Opcode::OpImm,
                        mnemonic: "addi".to_string(),
                        rd: rd as u8,
                        rs1: 0, // x0
                        imm,
                        funct3: 0,
                        funct7: 0,
                    }),
                })
            }
            0x3 => {
                // c.lui or c.addi16sp
                let rd = (inst >> 7) & 0x1F;

                if rd == 2 {
                    // c.addi16sp → addi x2, x2, nzimm[9:4]
                    let nzimm = extract_ci_addi16sp_immediate(inst);

                    if nzimm == 0 {
                        return Err(DecodeError::Reserved);
                    }

                    Ok(DecodedInstruction::Compressed {
                        raw: inst,
                        compressed_format: CompressedFormat::CI,
                        compressed_mnemonic: "c.addi16sp".to_string(),
                        expanded: Box::new(DecodedInstruction::IType {
                            raw: expand_ci_to_addi16sp(inst),
                            opcode: Opcode::OpImm,
                            mnemonic: "addi".to_string(),
                            rd: 2,  // x2 (stack pointer)
                            rs1: 2, // x2 (stack pointer)
                            imm: nzimm,
                            funct3: 0,
                            funct7: 0,
                        }),
                    })
                } else if rd != 0 {
                    // c.lui → lui rd, nzimm[17:12]
                    let nzimm = extract_ci_lui_immediate(inst);

                    if nzimm == 0 {
                        return Err(DecodeError::Reserved);
                    }

                    Ok(DecodedInstruction::Compressed {
                        raw: inst,
                        compressed_format: CompressedFormat::CI,
                        compressed_mnemonic: "c.lui".to_string(),
                        expanded: Box::new(DecodedInstruction::UType {
                            raw: expand_ci_to_lui(inst),
                            opcode: Opcode::Lui,
                            mnemonic: "lui".to_string(),
                            rd: rd as u8,
                            imm: nzimm,
                        }),
                    })
                } else {
                    // rd == 0 is reserved
                    Err(DecodeError::Reserved)
                }
            }
            0x4 => {
                // Complex arithmetic/shift operations based on inst[11:10]
                let sub_funct = (inst >> 10) & 0x3;
                let rd_prime = (inst >> 7) & 0x7;

                match sub_funct {
                    0x0 => {
                        // c.srli rd', shamt → srli rd', rd', shamt
                        let shamt = extract_cb_shift_immediate(inst);
                        if self.xlen == XLen::X32 && (shamt & 0x20) != 0 {
                            return Err(DecodeError::Reserved);
                        }

                        Ok(DecodedInstruction::Compressed {
                            raw: inst,
                            compressed_format: CompressedFormat::CB,
                            compressed_mnemonic: "c.srli".to_string(),
                            expanded: Box::new(DecodedInstruction::IType {
                                raw: expand_cb_to_srli(inst),
                                opcode: Opcode::OpImm,
                                mnemonic: "srli".to_string(),
                                rd: convert_compressed_reg(rd_prime as u8),
                                rs1: convert_compressed_reg(rd_prime as u8),
                                imm: shamt,
                                funct3: 5,
                                funct7: 0,
                            }),
                        })
                    }
                    0x1 => {
                        // c.srai rd', shamt → srai rd', rd', shamt
                        let shamt = extract_cb_shift_immediate(inst);
                        if self.xlen == XLen::X32 && (shamt & 0x20) != 0 {
                            return Err(DecodeError::Reserved);
                        }

                        Ok(DecodedInstruction::Compressed {
                            raw: inst,
                            compressed_format: CompressedFormat::CB,
                            compressed_mnemonic: "c.srai".to_string(),
                            expanded: Box::new(DecodedInstruction::IType {
                                raw: expand_cb_to_srai(inst),
                                opcode: Opcode::OpImm,
                                mnemonic: "srai".to_string(),
                                rd: convert_compressed_reg(rd_prime as u8),
                                rs1: convert_compressed_reg(rd_prime as u8),
                                imm: shamt,
                                funct3: 5,
                                funct7: 16, // 0x10 for srai
                            }),
                        })
                    }
                    0x2 => {
                        // c.andi rd', imm → andi rd', rd', imm
                        let imm = extract_cb_andi_immediate(inst);

                        Ok(DecodedInstruction::Compressed {
                            raw: inst,
                            compressed_format: CompressedFormat::CB,
                            compressed_mnemonic: "c.andi".to_string(),
                            expanded: Box::new(DecodedInstruction::IType {
                                raw: expand_cb_to_andi(inst),
                                opcode: Opcode::OpImm,
                                mnemonic: "andi".to_string(),
                                rd: convert_compressed_reg(rd_prime as u8),
                                rs1: convert_compressed_reg(rd_prime as u8),
                                imm,
                                funct3: 7,
                                funct7: 0,
                            }),
                        })
                    }
                    0x3 => {
                        // Register-Register operations based on inst[12] and inst[6:5]
                        let bit_12 = (inst >> 12) & 0x1;
                        let rs2_prime = (inst >> 2) & 0x7;
                        let sub_op = (inst >> 5) & 0x3;

                        if bit_12 == 0 {
                            // RV32/64 operations
                            match sub_op {
                                0x0 => {
                                    // c.sub rd', rs2' → sub rd', rd', rs2'
                                    Ok(DecodedInstruction::Compressed {
                                        raw: inst,
                                        compressed_format: CompressedFormat::CA,
                                        compressed_mnemonic: "c.sub".to_string(),
                                        expanded: Box::new(DecodedInstruction::RType {
                                            raw: expand_ca_to_sub(inst),
                                            opcode: Opcode::Op,
                                            mnemonic: "sub".to_string(),
                                            rd: convert_compressed_reg(rd_prime as u8),
                                            rs1: convert_compressed_reg(rd_prime as u8),
                                            rs2: convert_compressed_reg(rs2_prime as u8),
                                            funct3: 0,
                                            funct7: 32, // 0x20 for sub
                                        }),
                                    })
                                }
                                0x1 => {
                                    // c.xor rd', rs2' → xor rd', rd', rs2'
                                    Ok(DecodedInstruction::Compressed {
                                        raw: inst,
                                        compressed_format: CompressedFormat::CA,
                                        compressed_mnemonic: "c.xor".to_string(),
                                        expanded: Box::new(DecodedInstruction::RType {
                                            raw: expand_ca_to_xor(inst),
                                            opcode: Opcode::Op,
                                            mnemonic: "xor".to_string(),
                                            rd: convert_compressed_reg(rd_prime as u8),
                                            rs1: convert_compressed_reg(rd_prime as u8),
                                            rs2: convert_compressed_reg(rs2_prime as u8),
                                            funct3: 4,
                                            funct7: 0,
                                        }),
                                    })
                                }
                                0x2 => {
                                    // c.or rd', rs2' → or rd', rd', rs2'
                                    Ok(DecodedInstruction::Compressed {
                                        raw: inst,
                                        compressed_format: CompressedFormat::CA,
                                        compressed_mnemonic: "c.or".to_string(),
                                        expanded: Box::new(DecodedInstruction::RType {
                                            raw: expand_ca_to_or(inst),
                                            opcode: Opcode::Op,
                                            mnemonic: "or".to_string(),
                                            rd: convert_compressed_reg(rd_prime as u8),
                                            rs1: convert_compressed_reg(rd_prime as u8),
                                            rs2: convert_compressed_reg(rs2_prime as u8),
                                            funct3: 6,
                                            funct7: 0,
                                        }),
                                    })
                                }
                                0x3 => {
                                    // c.and rd', rs2' → and rd', rd', rs2'
                                    Ok(DecodedInstruction::Compressed {
                                        raw: inst,
                                        compressed_format: CompressedFormat::CA,
                                        compressed_mnemonic: "c.and".to_string(),
                                        expanded: Box::new(DecodedInstruction::RType {
                                            raw: expand_ca_to_and(inst),
                                            opcode: Opcode::Op,
                                            mnemonic: "and".to_string(),
                                            rd: convert_compressed_reg(rd_prime as u8),
                                            rs1: convert_compressed_reg(rd_prime as u8),
                                            rs2: convert_compressed_reg(rs2_prime as u8),
                                            funct3: 7,
                                            funct7: 0,
                                        }),
                                    })
                                }
                                _ => unreachable!("sub_op & 0x3 can only be 0-3"),
                            }
                        } else {
                            // RV64 operations (bit_12 == 1)
                            match sub_op {
                                0x0 => {
                                    // c.subw rd', rs2' → subw rd', rd', rs2'
                                    Ok(DecodedInstruction::Compressed {
                                        raw: inst,
                                        compressed_format: CompressedFormat::CA,
                                        compressed_mnemonic: "c.subw".to_string(),
                                        expanded: Box::new(DecodedInstruction::RType {
                                            raw: expand_ca_to_subw(inst),
                                            opcode: Opcode::Op32,
                                            mnemonic: "subw".to_string(),
                                            rd: convert_compressed_reg(rd_prime as u8),
                                            rs1: convert_compressed_reg(rd_prime as u8),
                                            rs2: convert_compressed_reg(rs2_prime as u8),
                                            funct3: 0,
                                            funct7: 32, // 0x20 for subw
                                        }),
                                    })
                                }
                                0x1 => {
                                    // c.addw rd', rs2' → addw rd', rd', rs2'
                                    Ok(DecodedInstruction::Compressed {
                                        raw: inst,
                                        compressed_format: CompressedFormat::CA,
                                        compressed_mnemonic: "c.addw".to_string(),
                                        expanded: Box::new(DecodedInstruction::RType {
                                            raw: expand_ca_to_addw(inst),
                                            opcode: Opcode::Op32,
                                            mnemonic: "addw".to_string(),
                                            rd: convert_compressed_reg(rd_prime as u8),
                                            rs1: convert_compressed_reg(rd_prime as u8),
                                            rs2: convert_compressed_reg(rs2_prime as u8),
                                            funct3: 0,
                                            funct7: 0,
                                        }),
                                    })
                                }
                                0x2 | 0x3 => {
                                    // Reserved
                                    Err(DecodeError::Reserved)
                                }
                                _ => unreachable!("sub_op & 0x3 can only be 0-3"),
                            }
                        }
                    }
                    _ => unreachable!("sub_funct & 0x3 can only be 0-3"),
                }
            }
            0x5 => {
                // c.j offset → jal x0, offset
                let offset = extract_cj_jump_immediate(inst);

                Ok(DecodedInstruction::Compressed {
                    raw: inst,
                    compressed_format: CompressedFormat::CJ,
                    compressed_mnemonic: "c.j".to_string(),
                    expanded: Box::new(DecodedInstruction::JType {
                        raw: expand_cj_to_jal(inst),
                        opcode: Opcode::Jal,
                        mnemonic: "jal".to_string(),
                        rd: 0, // x0
                        imm: offset,
                    }),
                })
            }
            0x6 => {
                // c.beqz rs1', offset → beq rs1', x0, offset
                let rs1_prime = (inst >> 7) & 0x7;
                let offset = extract_cb_branch_immediate(inst);

                Ok(DecodedInstruction::Compressed {
                    raw: inst,
                    compressed_format: CompressedFormat::CB,
                    compressed_mnemonic: "c.beqz".to_string(),
                    expanded: Box::new(DecodedInstruction::BType {
                        raw: expand_cb_to_beq(inst),
                        opcode: Opcode::Branch,
                        mnemonic: "beq".to_string(),
                        rs1: convert_compressed_reg(rs1_prime as u8),
                        rs2: 0, // x0
                        imm: offset,
                        funct3: 0,
                    }),
                })
            }
            0x7 => {
                // c.bnez rs1', offset → bne rs1', x0, offset
                let rs1_prime = (inst >> 7) & 0x7;
                let offset = extract_cb_branch_immediate(inst);

                Ok(DecodedInstruction::Compressed {
                    raw: inst,
                    compressed_format: CompressedFormat::CB,
                    compressed_mnemonic: "c.bnez".to_string(),
                    expanded: Box::new(DecodedInstruction::BType {
                        raw: expand_cb_to_bne(inst),
                        opcode: Opcode::Branch,
                        mnemonic: "bne".to_string(),
                        rs1: convert_compressed_reg(rs1_prime as u8),
                        rs2: 0, // x0
                        imm: offset,
                        funct3: 1,
                    }),
                })
            }
            _ => Err(DecodeError::InvalidProgram(
                "Quadrant 1 instruction not yet implemented".to_string(),
            )),
        }
    }
}

// Immediate extraction functions for different compressed formats
fn extract_ci_lui_immediate(inst: u16) -> i32 {
    // CI immediate for c.lui: nzimm[17:12]
    let imm_5 = (inst >> 12) & 0x1;
    let imm_4_0 = (inst >> 2) & 0x1F;

    let imm = (imm_5 << 5) | imm_4_0;

    // Sign extend 6-bit immediate and shift to upper 20 bits for LUI
    if imm & 0x20 != 0 {
        ((imm as i32) - 64) << 12
    } else {
        (imm as i32) << 12
    }
}

fn extract_ci_addi16sp_immediate(inst: u16) -> i32 {
    // CI immediate for c.addi16sp: nzimm[9|4|6|8:7|5]
    let imm_9 = (inst >> 12) & 0x1;
    let imm_4 = (inst >> 6) & 0x1;
    let imm_6 = (inst >> 5) & 0x1;
    let imm_8_7 = (inst >> 3) & 0x3;
    let imm_5 = (inst >> 2) & 0x1;

    let imm = (imm_9 << 9) | (imm_8_7 << 7) | (imm_6 << 6) | (imm_5 << 5) | (imm_4 << 4);

    // Sign extend 10-bit immediate
    if imm & 0x200 != 0 {
        (imm as i32) - 1024
    } else {
        imm as i32
    }
}

fn extract_cb_shift_immediate(inst: u16) -> i32 {
    // CB immediate for c.srli/c.srai: shamt[5:0]
    let shamt_5 = (inst >> 12) & 0x1;
    let shamt_4_0 = (inst >> 2) & 0x1F;

    ((shamt_5 << 5) | shamt_4_0) as i32
}

fn extract_cb_andi_immediate(inst: u16) -> i32 {
    // CB immediate for c.andi: imm[5:0]
    let imm_5 = (inst >> 12) & 0x1;
    let imm_4_0 = (inst >> 2) & 0x1F;

    let imm = (imm_5 << 5) | imm_4_0;

    // Sign extend 6-bit immediate
    if imm & 0x20 != 0 {
        (imm as i32) - 64
    } else {
        imm as i32
    }
}

fn extract_cj_jump_immediate(inst: u16) -> i32 {
    // CJ immediate for c.j: offset[11|4|9:8|10|6|7|3:1|5]
    let offset_11 = (inst >> 12) & 0x1;
    let offset_4 = (inst >> 11) & 0x1;
    let offset_9_8 = (inst >> 9) & 0x3;
    let offset_10 = (inst >> 8) & 0x1;
    let offset_6 = (inst >> 7) & 0x1;
    let offset_7 = (inst >> 6) & 0x1;
    let offset_3_1 = (inst >> 3) & 0x7;
    let offset_5 = (inst >> 2) & 0x1;

    let offset = (offset_11 << 11)
        | (offset_10 << 10)
        | (offset_9_8 << 8)
        | (offset_7 << 7)
        | (offset_6 << 6)
        | (offset_5 << 5)
        | (offset_4 << 4)
        | (offset_3_1 << 1);

    // Sign extend 12-bit immediate
    if offset & 0x800 != 0 {
        (offset as i32) - 4096
    } else {
        offset as i32
    }
}

fn extract_cb_branch_immediate(inst: u16) -> i32 {
    // CB immediate for c.beqz/c.bnez: offset[8|4:3|7:6|2:1|5]
    let offset_8 = (inst >> 12) & 0x1;
    let offset_4_3 = (inst >> 10) & 0x3;
    let offset_7_6 = (inst >> 5) & 0x3;
    let offset_2_1 = (inst >> 3) & 0x3;
    let offset_5 = (inst >> 2) & 0x1;

    let offset = (offset_8 << 8)
        | (offset_7_6 << 6)
        | (offset_5 << 5)
        | (offset_4_3 << 3)
        | (offset_2_1 << 1);

    // Sign extend 9-bit immediate
    if offset & 0x100 != 0 {
        (offset as i32) - 512
    } else {
        offset as i32
    }
}

fn extract_ciw_immediate(inst: u16) -> i32 {
    // CIW immediate for c.addi4spn: nzuimm[9:2]
    let imm_5_4 = (inst >> 11) & 0x3;
    let imm_9_6 = (inst >> 7) & 0xF;
    let imm_2 = (inst >> 6) & 0x1;
    let imm_3 = (inst >> 5) & 0x1;

    ((imm_9_6 << 6) | (imm_5_4 << 4) | (imm_3 << 3) | (imm_2 << 2)) as i32
}

fn extract_ci_addi_immediate(inst: u16) -> i32 {
    // CI immediate: imm[5] | imm[4:0]
    let imm_5 = (inst >> 12) & 0x1;
    let imm_4_0 = (inst >> 2) & 0x1F;

    let imm = (imm_5 << 5) | imm_4_0;

    // Sign extend 6-bit immediate
    if imm & 0x20 != 0 {
        (imm as i32) - 64
    } else {
        imm as i32
    }
}

fn extract_cl_lw_immediate(inst: u16) -> i32 {
    // CL immediate for c.lw: offset[6:2]
    let offset_6 = (inst >> 5) & 0x1;
    let offset_2 = (inst >> 6) & 0x1;
    let offset_5_3 = (inst >> 10) & 0x7;

    ((offset_6 << 6) | (offset_5_3 << 3) | (offset_2 << 2)) as i32
}

fn extract_cl_ld_immediate(inst: u16) -> i32 {
    // CL immediate for c.ld: offset[7:3]
    let offset_7_6 = (inst >> 5) & 0x3;
    let offset_5_3 = (inst >> 10) & 0x7;

    ((offset_7_6 << 6) | (offset_5_3 << 3)) as i32
}

fn extract_cs_sw_immediate(inst: u16) -> i32 {
    // Same as CL lw immediate
    extract_cl_lw_immediate(inst)
}

fn extract_cs_sd_immediate(inst: u16) -> i32 {
    // Same as CL ld immediate
    extract_cl_ld_immediate(inst)
}

// Expansion functions to create equivalent 32-bit instructions
fn expand_ciw_to_addi(inst: u16) -> u32 {
    let rd_prime = (inst >> 2) & 0x7;
    let nzuimm = extract_ciw_immediate(inst);

    // addi rd', x2, nzuimm
    0x00000013 // addi opcode
        | ((convert_compressed_reg(rd_prime as u8) as u32) << 7)  // rd
        | (2u32 << 15) // rs1 = x2 (stack pointer)
        | ((nzuimm as u32) << 20) // immediate
}

fn expand_ci_to_addi(inst: u16) -> u32 {
    let rd = (inst >> 7) & 0x1F;
    let imm = extract_ci_addi_immediate(inst);

    // addi rd, rd, imm
    0x00000013 // addi opcode
        | ((rd as u32) << 7)  // rd
        | ((rd as u32) << 15) // rs1 = rd
        | (((imm as u32) & 0xFFF) << 20) // immediate
}

fn expand_ci_to_addiw(inst: u16) -> u32 {
    let rd = (inst >> 7) & 0x1F;
    let imm = extract_ci_addi_immediate(inst);

    // addiw rd, rd, imm
    0x0000001B // addiw opcode
        | ((rd as u32) << 7)  // rd
        | ((rd as u32) << 15) // rs1 = rd
        | (((imm as u32) & 0xFFF) << 20) // immediate
}

fn expand_ci_to_li(inst: u16) -> u32 {
    let rd = (inst >> 7) & 0x1F;
    let imm = extract_ci_addi_immediate(inst);

    // addi rd, x0, imm
    0x00000013 // addi opcode
        | ((rd as u32) << 7)  // rd
        | (0u32 << 15) // rs1 = x0
        | (((imm as u32) & 0xFFF) << 20) // immediate
}

fn expand_cl_to_lw(inst: u16) -> u32 {
    let rd_prime = (inst >> 2) & 0x7;
    let rs1_prime = (inst >> 7) & 0x7;
    let offset = extract_cl_lw_immediate(inst);

    // lw rd', offset(rs1')
    0x00000003 // load opcode
        | ((convert_compressed_reg(rd_prime as u8) as u32) << 7)  // rd
        | (2u32 << 12) // funct3 = 2 (lw)
        | ((convert_compressed_reg(rs1_prime as u8) as u32) << 15) // rs1
        | (((offset as u32) & 0xFFF) << 20) // immediate
}

fn expand_cl_to_ld(inst: u16) -> u32 {
    let rd_prime = (inst >> 2) & 0x7;
    let rs1_prime = (inst >> 7) & 0x7;
    let offset = extract_cl_ld_immediate(inst);

    // ld rd', offset(rs1')
    0x00000003 // load opcode
        | ((convert_compressed_reg(rd_prime as u8) as u32) << 7)  // rd
        | (3u32 << 12) // funct3 = 3 (ld)
        | ((convert_compressed_reg(rs1_prime as u8) as u32) << 15) // rs1
        | (((offset as u32) & 0xFFF) << 20) // immediate
}

fn expand_cs_to_sw(inst: u16) -> u32 {
    let rs2_prime = (inst >> 2) & 0x7;
    let rs1_prime = (inst >> 7) & 0x7;
    let offset = extract_cs_sw_immediate(inst);

    let imm_4_0 = (offset as u32) & 0x1F;
    let imm_11_5 = ((offset as u32) >> 5) & 0x7F;

    // sw rs2', offset(rs1')
    0x00000023 // store opcode
        | (imm_4_0 << 7) // imm[4:0]
        | (2u32 << 12) // funct3 = 2 (sw)
        | ((convert_compressed_reg(rs1_prime as u8) as u32) << 15) // rs1
        | ((convert_compressed_reg(rs2_prime as u8) as u32) << 20) // rs2
        | (imm_11_5 << 25) // imm[11:5]
}

fn expand_cs_to_sd(inst: u16) -> u32 {
    let rs2_prime = (inst >> 2) & 0x7;
    let rs1_prime = (inst >> 7) & 0x7;
    let offset = extract_cs_sd_immediate(inst);

    let imm_4_0 = (offset as u32) & 0x1F;
    let imm_11_5 = ((offset as u32) >> 5) & 0x7F;

    // sd rs2', offset(rs1')
    0x00000023 // store opcode
        | (imm_4_0 << 7) // imm[4:0]
        | (3u32 << 12) // funct3 = 3 (sd)
        | ((convert_compressed_reg(rs1_prime as u8) as u32) << 15) // rs1
        | ((convert_compressed_reg(rs2_prime as u8) as u32) << 20) // rs2
        | (imm_11_5 << 25) // imm[11:5]
}

// Additional expansion functions for new Quadrant 1 instructions
fn expand_ci_to_lui(inst: u16) -> u32 {
    let rd = (inst >> 7) & 0x1F;
    let imm = extract_ci_lui_immediate(inst);

    // lui rd, imm
    0x00000037 // lui opcode
        | ((rd as u32) << 7) // rd
        | (((imm as u32) & 0xFFFFF000) << 0) // immediate[31:12] already shifted
}

fn expand_ci_to_addi16sp(inst: u16) -> u32 {
    let imm = extract_ci_addi16sp_immediate(inst);

    // addi x2, x2, imm
    0x00000013 // addi opcode
        | (2u32 << 7)  // rd = x2
        | (2u32 << 15) // rs1 = x2
        | (((imm as u32) & 0xFFF) << 20) // immediate
}

fn expand_cb_to_srli(inst: u16) -> u32 {
    let rd_prime = (inst >> 7) & 0x7;
    let shamt = extract_cb_shift_immediate(inst);

    // srli rd', rd', shamt
    0x00000013 // OP-IMM opcode
        | ((convert_compressed_reg(rd_prime as u8) as u32) << 7)  // rd
        | (5u32 << 12) // funct3 = 5 (srli)
        | ((convert_compressed_reg(rd_prime as u8) as u32) << 15) // rs1
        | (((shamt as u32) & 0x3F) << 20) // shamt[5:0]
        | (0u32 << 26) // funct7[6:0] = 0000000 for srli
}

fn expand_cb_to_srai(inst: u16) -> u32 {
    let rd_prime = (inst >> 7) & 0x7;
    let shamt = extract_cb_shift_immediate(inst);

    // srai rd', rd', shamt
    0x00000013 // OP-IMM opcode
        | ((convert_compressed_reg(rd_prime as u8) as u32) << 7)  // rd
        | (5u32 << 12) // funct3 = 5 (srai)
        | ((convert_compressed_reg(rd_prime as u8) as u32) << 15) // rs1
        | (((shamt as u32) & 0x3F) << 20) // shamt[5:0]
        | (16u32 << 26) // funct7[6:0] = 0100000 for srai
}

fn expand_cb_to_andi(inst: u16) -> u32 {
    let rd_prime = (inst >> 7) & 0x7;
    let imm = extract_cb_andi_immediate(inst);

    // andi rd', rd', imm
    0x00000013 // OP-IMM opcode
        | ((convert_compressed_reg(rd_prime as u8) as u32) << 7)  // rd
        | (7u32 << 12) // funct3 = 7 (andi)
        | ((convert_compressed_reg(rd_prime as u8) as u32) << 15) // rs1
        | (((imm as u32) & 0xFFF) << 20) // immediate
}

fn expand_ca_to_sub(inst: u16) -> u32 {
    let rd_prime = (inst >> 7) & 0x7;
    let rs2_prime = (inst >> 2) & 0x7;

    // sub rd', rd', rs2'
    0x00000033 // OP opcode
        | ((convert_compressed_reg(rd_prime as u8) as u32) << 7)  // rd
        | (0u32 << 12) // funct3 = 0 (sub)
        | ((convert_compressed_reg(rd_prime as u8) as u32) << 15) // rs1
        | ((convert_compressed_reg(rs2_prime as u8) as u32) << 20) // rs2
        | (32u32 << 25) // funct7 = 0100000 for sub
}

fn expand_ca_to_xor(inst: u16) -> u32 {
    let rd_prime = (inst >> 7) & 0x7;
    let rs2_prime = (inst >> 2) & 0x7;

    // xor rd', rd', rs2'
    0x00000033 // OP opcode
        | ((convert_compressed_reg(rd_prime as u8) as u32) << 7)  // rd
        | (4u32 << 12) // funct3 = 4 (xor)
        | ((convert_compressed_reg(rd_prime as u8) as u32) << 15) // rs1
        | ((convert_compressed_reg(rs2_prime as u8) as u32) << 20) // rs2
        | (0u32 << 25) // funct7 = 0000000 for xor
}

fn expand_ca_to_or(inst: u16) -> u32 {
    let rd_prime = (inst >> 7) & 0x7;
    let rs2_prime = (inst >> 2) & 0x7;

    // or rd', rd', rs2'
    0x00000033 // OP opcode
        | ((convert_compressed_reg(rd_prime as u8) as u32) << 7)  // rd
        | (6u32 << 12) // funct3 = 6 (or)
        | ((convert_compressed_reg(rd_prime as u8) as u32) << 15) // rs1
        | ((convert_compressed_reg(rs2_prime as u8) as u32) << 20) // rs2
        | (0u32 << 25) // funct7 = 0000000 for or
}

fn expand_ca_to_and(inst: u16) -> u32 {
    let rd_prime = (inst >> 7) & 0x7;
    let rs2_prime = (inst >> 2) & 0x7;

    // and rd', rd', rs2'
    0x00000033 // OP opcode
        | ((convert_compressed_reg(rd_prime as u8) as u32) << 7)  // rd
        | (7u32 << 12) // funct3 = 7 (and)
        | ((convert_compressed_reg(rd_prime as u8) as u32) << 15) // rs1
        | ((convert_compressed_reg(rs2_prime as u8) as u32) << 20) // rs2
        | (0u32 << 25) // funct7 = 0000000 for and
}

fn expand_ca_to_subw(inst: u16) -> u32 {
    let rd_prime = (inst >> 7) & 0x7;
    let rs2_prime = (inst >> 2) & 0x7;

    // subw rd', rd', rs2'
    0x0000003B // OP-32 opcode
        | ((convert_compressed_reg(rd_prime as u8) as u32) << 7)  // rd
        | (0u32 << 12) // funct3 = 0 (subw)
        | ((convert_compressed_reg(rd_prime as u8) as u32) << 15) // rs1
        | ((convert_compressed_reg(rs2_prime as u8) as u32) << 20) // rs2
        | (32u32 << 25) // funct7 = 0100000 for subw
}

fn expand_ca_to_addw(inst: u16) -> u32 {
    let rd_prime = (inst >> 7) & 0x7;
    let rs2_prime = (inst >> 2) & 0x7;

    // addw rd', rd', rs2'
    0x0000003B // OP-32 opcode
        | ((convert_compressed_reg(rd_prime as u8) as u32) << 7)  // rd
        | (0u32 << 12) // funct3 = 0 (addw)
        | ((convert_compressed_reg(rd_prime as u8) as u32) << 15) // rs1
        | ((convert_compressed_reg(rs2_prime as u8) as u32) << 20) // rs2
        | (0u32 << 25) // funct7 = 0000000 for addw
}

fn expand_cj_to_jal(inst: u16) -> u32 {
    let offset = extract_cj_jump_immediate(inst);

    // jal x0, offset
    let imm_20 = (offset >> 20) & 0x1;
    let imm_10_1 = (offset >> 1) & 0x3FF;
    let imm_11 = (offset >> 11) & 0x1;
    let imm_19_12 = (offset >> 12) & 0xFF;

    0x0000006F // JAL opcode
        | (0u32 << 7) // rd = x0
        | ((imm_19_12 as u32) << 12) // imm[19:12]
        | ((imm_11 as u32) << 20) // imm[11]
        | ((imm_10_1 as u32) << 21) // imm[10:1]
        | ((imm_20 as u32) << 31) // imm[20]
}

fn expand_cb_to_beq(inst: u16) -> u32 {
    let rs1_prime = (inst >> 7) & 0x7;
    let offset = extract_cb_branch_immediate(inst);

    // beq rs1', x0, offset
    let imm_12 = (offset >> 12) & 0x1;
    let imm_10_5 = (offset >> 5) & 0x3F;
    let imm_4_1 = (offset >> 1) & 0xF;
    let imm_11 = (offset >> 11) & 0x1;

    0x00000063 // BRANCH opcode
        | ((imm_11 as u32) << 7) // imm[11]
        | ((imm_4_1 as u32) << 8) // imm[4:1]
        | (0u32 << 12) // funct3 = 0 (beq)
        | ((convert_compressed_reg(rs1_prime as u8) as u32) << 15) // rs1
        | (0u32 << 20) // rs2 = x0
        | ((imm_10_5 as u32) << 25) // imm[10:5]
        | ((imm_12 as u32) << 31) // imm[12]
}

fn expand_cb_to_bne(inst: u16) -> u32 {
    let rs1_prime = (inst >> 7) & 0x7;
    let offset = extract_cb_branch_immediate(inst);

    // bne rs1', x0, offset
    let imm_12 = (offset >> 12) & 0x1;
    let imm_10_5 = (offset >> 5) & 0x3F;
    let imm_4_1 = (offset >> 1) & 0xF;
    let imm_11 = (offset >> 11) & 0x1;

    0x00000063 // BRANCH opcode
        | ((imm_11 as u32) << 7) // imm[11]
        | ((imm_4_1 as u32) << 8) // imm[4:1]
        | (1u32 << 12) // funct3 = 1 (bne)
        | ((convert_compressed_reg(rs1_prime as u8) as u32) << 15) // rs1
        | (0u32 << 20) // rs2 = x0
        | ((imm_10_5 as u32) << 25) // imm[10:5]
        | ((imm_12 as u32) << 31) // imm[12]
}

// Additional immediate extraction functions for Quadrant 2
fn extract_ci_slli_immediate(inst: u16) -> i32 {
    // CI immediate for c.slli: shamt[5:0]
    let shamt_5 = (inst >> 12) & 0x1;
    let shamt_4_0 = (inst >> 2) & 0x1F;

    ((shamt_5 << 5) | shamt_4_0) as i32
}

fn extract_ci_lwsp_immediate(inst: u16) -> i32 {
    // CI immediate for c.lwsp: offset[5|4:2|7:6]
    let offset_5 = (inst >> 12) & 0x1;
    let offset_4_2 = (inst >> 4) & 0x7;
    let offset_7_6 = (inst >> 2) & 0x3;

    ((offset_7_6 << 6) | (offset_5 << 5) | (offset_4_2 << 2)) as i32
}

fn extract_ci_ldsp_immediate(inst: u16) -> i32 {
    // CI immediate for c.ldsp: offset[5|4:3|8:6]
    let offset_5 = (inst >> 12) & 0x1;
    let offset_4_3 = (inst >> 5) & 0x3;
    let offset_8_6 = (inst >> 2) & 0x7;

    ((offset_8_6 << 6) | (offset_5 << 5) | (offset_4_3 << 3)) as i32
}

fn extract_css_swsp_immediate(inst: u16) -> i32 {
    // CSS immediate for c.swsp: offset[5:2|7:6]
    let offset_5_2 = (inst >> 9) & 0xF;
    let offset_7_6 = (inst >> 7) & 0x3;

    ((offset_7_6 << 6) | (offset_5_2 << 2)) as i32
}

fn extract_css_sdsp_immediate(inst: u16) -> i32 {
    // CSS immediate for c.sdsp: offset[5:3|8:6]
    let offset_5_3 = (inst >> 10) & 0x7;
    let offset_8_6 = (inst >> 7) & 0x7;

    ((offset_8_6 << 6) | (offset_5_3 << 3)) as i32
}

// Additional expansion functions for Quadrant 2
fn expand_ci_to_slli(inst: u16) -> u32 {
    let rd = (inst >> 7) & 0x1F;
    let shamt = extract_ci_slli_immediate(inst);

    // slli rd, rd, shamt
    0x00000013 // OP-IMM opcode
        | ((rd as u32) << 7)  // rd
        | (1u32 << 12) // funct3 = 1 (slli)
        | ((rd as u32) << 15) // rs1 = rd
        | (((shamt as u32) & 0x3F) << 20) // shamt[5:0]
        | (0u32 << 26) // funct7[6:0] = 0000000 for slli
}

fn expand_ci_to_lwsp(inst: u16) -> u32 {
    let rd = (inst >> 7) & 0x1F;
    let offset = extract_ci_lwsp_immediate(inst);

    // lw rd, offset(x2)
    0x00000003 // load opcode
        | ((rd as u32) << 7)  // rd
        | (2u32 << 12) // funct3 = 2 (lw)
        | (2u32 << 15) // rs1 = x2 (stack pointer)
        | (((offset as u32) & 0xFFF) << 20) // immediate
}

fn expand_ci_to_ldsp(inst: u16) -> u32 {
    let rd = (inst >> 7) & 0x1F;
    let offset = extract_ci_ldsp_immediate(inst);

    // ld rd, offset(x2)
    0x00000003 // load opcode
        | ((rd as u32) << 7)  // rd
        | (3u32 << 12) // funct3 = 3 (ld)
        | (2u32 << 15) // rs1 = x2 (stack pointer)
        | (((offset as u32) & 0xFFF) << 20) // immediate
}

fn expand_cr_to_jr(inst: u16) -> u32 {
    let rs1 = (inst >> 7) & 0x1F;

    // jalr x0, 0(rs1)
    0x00000067 // jalr opcode
        | (0u32 << 7)  // rd = x0
        | (0u32 << 12) // funct3 = 0
        | ((rs1 as u32) << 15) // rs1
        | (0u32 << 20) // immediate = 0
}

fn expand_cr_to_mv(inst: u16) -> u32 {
    let rd = (inst >> 7) & 0x1F;
    let rs2 = (inst >> 2) & 0x1F;

    // add rd, x0, rs2
    0x00000033 // OP opcode
        | ((rd as u32) << 7)  // rd
        | (0u32 << 12) // funct3 = 0 (add)
        | (0u32 << 15) // rs1 = x0
        | ((rs2 as u32) << 20) // rs2
        | (0u32 << 25) // funct7 = 0000000 for add
}

fn expand_cr_to_jalr(inst: u16) -> u32 {
    let rs1 = (inst >> 7) & 0x1F;

    // jalr x1, 0(rs1)
    0x00000067 // jalr opcode
        | (1u32 << 7)  // rd = x1 (return address)
        | (0u32 << 12) // funct3 = 0
        | ((rs1 as u32) << 15) // rs1
        | (0u32 << 20) // immediate = 0
}

fn expand_cr_to_add(inst: u16) -> u32 {
    let rd = (inst >> 7) & 0x1F;
    let rs2 = (inst >> 2) & 0x1F;

    // add rd, rd, rs2
    0x00000033 // OP opcode
        | ((rd as u32) << 7)  // rd
        | (0u32 << 12) // funct3 = 0 (add)
        | ((rd as u32) << 15) // rs1 = rd
        | ((rs2 as u32) << 20) // rs2
        | (0u32 << 25) // funct7 = 0000000 for add
}

fn expand_css_to_swsp(inst: u16) -> u32 {
    let rs2 = (inst >> 2) & 0x1F;
    let offset = extract_css_swsp_immediate(inst);

    let imm_4_0 = (offset as u32) & 0x1F;
    let imm_11_5 = ((offset as u32) >> 5) & 0x7F;

    // sw rs2, offset(x2)
    0x00000023 // store opcode
        | (imm_4_0 << 7) // imm[4:0]
        | (2u32 << 12) // funct3 = 2 (sw)
        | (2u32 << 15) // rs1 = x2 (stack pointer)
        | ((rs2 as u32) << 20) // rs2
        | (imm_11_5 << 25) // imm[11:5]
}

fn expand_css_to_sdsp(inst: u16) -> u32 {
    let rs2 = (inst >> 2) & 0x1F;
    let offset = extract_css_sdsp_immediate(inst);

    let imm_4_0 = (offset as u32) & 0x1F;
    let imm_11_5 = ((offset as u32) >> 5) & 0x7F;

    // sd rs2, offset(x2)
    0x00000023 // store opcode
        | (imm_4_0 << 7) // imm[4:0]
        | (3u32 << 12) // funct3 = 3 (sd)
        | (2u32 << 15) // rs1 = x2 (stack pointer)
        | ((rs2 as u32) << 20) // rs2
        | (imm_11_5 << 25) // imm[11:5]
}

/// Decoder for Quadrant 2 compressed instructions (bits [1:0] = 10)  
pub struct Quadrant2Decoder {
    xlen: XLen,
}

impl Quadrant2Decoder {
    pub fn new(xlen: XLen) -> Self {
        Self { xlen }
    }
}

impl CompressedInstructionDecoder for Quadrant2Decoder {
    fn quadrant(&self) -> u8 {
        2
    }

    fn decode(&self, inst: u16) -> DecodeResult<DecodedInstruction> {
        let funct3 = (inst >> 13) & 0x7;

        match funct3 {
            0x0 => {
                // c.slli rd, shamt → slli rd, rd, shamt
                let rd = (inst >> 7) & 0x1F;
                let shamt = extract_ci_slli_immediate(inst);
                if self.xlen == XLen::X32 && (shamt & 0x20) != 0 {
                    return Err(DecodeError::Reserved);
                }

                if rd == 0 {
                    return Err(DecodeError::Reserved);
                }

                Ok(DecodedInstruction::Compressed {
                    raw: inst,
                    compressed_format: CompressedFormat::CI,
                    compressed_mnemonic: "c.slli".to_string(),
                    expanded: Box::new(DecodedInstruction::IType {
                        raw: expand_ci_to_slli(inst),
                        opcode: Opcode::OpImm,
                        mnemonic: "slli".to_string(),
                        rd: rd as u8,
                        rs1: rd as u8,
                        imm: shamt,
                        funct3: 1,
                        funct7: 0,
                    }),
                })
            }
            0x1 => {
                // c.fldsp (floating point - not supported)
                Err(DecodeError::Reserved)
            }
            0x2 => {
                // c.lwsp rd, offset → lw rd, offset(x2)
                let rd = (inst >> 7) & 0x1F;
                let offset = extract_ci_lwsp_immediate(inst);

                if rd == 0 {
                    return Err(DecodeError::Reserved);
                }

                Ok(DecodedInstruction::Compressed {
                    raw: inst,
                    compressed_format: CompressedFormat::CI,
                    compressed_mnemonic: "c.lwsp".to_string(),
                    expanded: Box::new(DecodedInstruction::IType {
                        raw: expand_ci_to_lwsp(inst),
                        opcode: Opcode::Load,
                        mnemonic: "lw".to_string(),
                        rd: rd as u8,
                        rs1: 2, // x2 (stack pointer)
                        imm: offset,
                        funct3: 2, // lw funct3
                        funct7: 0,
                    }),
                })
            }
            0x3 => {
                // c.ldsp rd, offset → ld rd, offset(x2)
                let rd = (inst >> 7) & 0x1F;
                let offset = extract_ci_ldsp_immediate(inst);

                if rd == 0 {
                    return Err(DecodeError::Reserved);
                }

                Ok(DecodedInstruction::Compressed {
                    raw: inst,
                    compressed_format: CompressedFormat::CI,
                    compressed_mnemonic: "c.ldsp".to_string(),
                    expanded: Box::new(DecodedInstruction::IType {
                        raw: expand_ci_to_ldsp(inst),
                        opcode: Opcode::Load,
                        mnemonic: "ld".to_string(),
                        rd: rd as u8,
                        rs1: 2, // x2 (stack pointer)
                        imm: offset,
                        funct3: 3, // ld funct3
                        funct7: 0,
                    }),
                })
            }
            0x4 => {
                // Complex encoding based on inst[12] and register fields
                let bit_12 = (inst >> 12) & 0x1;
                let rd = (inst >> 7) & 0x1F;
                let rs2 = (inst >> 2) & 0x1F;

                if bit_12 == 0 {
                    // When inst[12] = 0
                    if rs2 == 0 {
                        // c.jr rs1 → jalr x0, 0(rs1)
                        Ok(DecodedInstruction::Compressed {
                            raw: inst,
                            compressed_format: CompressedFormat::CR,
                            compressed_mnemonic: "c.jr".to_string(),
                            expanded: Box::new(DecodedInstruction::IType {
                                raw: expand_cr_to_jr(inst),
                                opcode: Opcode::Jalr,
                                mnemonic: "jalr".to_string(),
                                rd: 0,         // x0
                                rs1: rd as u8, // rs1 (from rd field)
                                imm: 0,
                                funct3: 0,
                                funct7: 0,
                            }),
                        })
                    } else {
                        // c.mv rd, rs2 → add rd, x0, rs2
                        Ok(DecodedInstruction::Compressed {
                            raw: inst,
                            compressed_format: CompressedFormat::CR,
                            compressed_mnemonic: "c.mv".to_string(),
                            expanded: Box::new(DecodedInstruction::RType {
                                raw: expand_cr_to_mv(inst),
                                opcode: Opcode::Op,
                                mnemonic: "add".to_string(),
                                rd: rd as u8,
                                rs1: 0, // x0
                                rs2: rs2 as u8,
                                funct3: 0,
                                funct7: 0,
                            }),
                        })
                    }
                } else {
                    // When inst[12] = 1
                    if rd == 0 && rs2 == 0 {
                        // c.ebreak → ebreak
                        Ok(DecodedInstruction::Compressed {
                            raw: inst,
                            compressed_format: CompressedFormat::CI,
                            compressed_mnemonic: "c.ebreak".to_string(),
                            expanded: Box::new(DecodedInstruction::System {
                                raw: 0x00100073, // ebreak instruction
                                opcode: Opcode::System,
                                mnemonic: "ebreak".to_string(),
                                rd: 0,
                                rs1: 0,
                                funct3: 0,
                                csr: 1, // ebreak has immediate=1
                            }),
                        })
                    } else if rs2 == 0 && rd != 0 {
                        // c.jalr rs1 → jalr x1, 0(rs1)
                        Ok(DecodedInstruction::Compressed {
                            raw: inst,
                            compressed_format: CompressedFormat::CR,
                            compressed_mnemonic: "c.jalr".to_string(),
                            expanded: Box::new(DecodedInstruction::IType {
                                raw: expand_cr_to_jalr(inst),
                                opcode: Opcode::Jalr,
                                mnemonic: "jalr".to_string(),
                                rd: 1,         // x1 (return address)
                                rs1: rd as u8, // rs1 (from rd field)
                                imm: 0,
                                funct3: 0,
                                funct7: 0,
                            }),
                        })
                    } else if rs2 != 0 {
                        // c.add rd, rs2 → add rd, rd, rs2
                        Ok(DecodedInstruction::Compressed {
                            raw: inst,
                            compressed_format: CompressedFormat::CR,
                            compressed_mnemonic: "c.add".to_string(),
                            expanded: Box::new(DecodedInstruction::RType {
                                raw: expand_cr_to_add(inst),
                                opcode: Opcode::Op,
                                mnemonic: "add".to_string(),
                                rd: rd as u8,
                                rs1: rd as u8, // c.add uses same reg for src/dest
                                rs2: rs2 as u8,
                                funct3: 0,
                                funct7: 0,
                            }),
                        })
                    } else {
                        Err(DecodeError::Reserved)
                    }
                }
            }
            0x5 => {
                // c.fsdsp (floating point - not supported)
                Err(DecodeError::Reserved)
            }
            0x6 => {
                // c.swsp rs2, offset → sw rs2, offset(x2)
                let rs2 = (inst >> 2) & 0x1F;
                let offset = extract_css_swsp_immediate(inst);

                Ok(DecodedInstruction::Compressed {
                    raw: inst,
                    compressed_format: CompressedFormat::CSS,
                    compressed_mnemonic: "c.swsp".to_string(),
                    expanded: Box::new(DecodedInstruction::SType {
                        raw: expand_css_to_swsp(inst),
                        opcode: Opcode::Store,
                        mnemonic: "sw".to_string(),
                        rs1: 2, // x2 (stack pointer)
                        rs2: rs2 as u8,
                        imm: offset,
                        funct3: 2, // sw funct3
                    }),
                })
            }
            0x7 => {
                // c.sdsp rs2, offset → sd rs2, offset(x2)
                let rs2 = (inst >> 2) & 0x1F;
                let offset = extract_css_sdsp_immediate(inst);

                Ok(DecodedInstruction::Compressed {
                    raw: inst,
                    compressed_format: CompressedFormat::CSS,
                    compressed_mnemonic: "c.sdsp".to_string(),
                    expanded: Box::new(DecodedInstruction::SType {
                        raw: expand_css_to_sdsp(inst),
                        opcode: Opcode::Store,
                        mnemonic: "sd".to_string(),
                        rs1: 2, // x2 (stack pointer)
                        rs2: rs2 as u8,
                        imm: offset,
                        funct3: 3, // sd funct3
                    }),
                })
            }
            _ => Err(DecodeError::InvalidProgram(format!("Quadrant 2, funct3={} invalid", funct3))),
        }
    }
}
