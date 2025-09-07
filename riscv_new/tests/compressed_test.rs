use riscv_new::{InstructionDecoder, Target};

#[test]
fn test_c_nop() {
    let decoder = InstructionDecoder::new();
    // C.NOP: 0x0001 (expands to addi x0, x0, 0)
    let bytes = vec![0x01, 0x00]; // 0x0001 in little endian
    let result = decoder.decode_bytes(&bytes).unwrap();
    assert_eq!(result.len(), 1);

    let (instruction, comp_type) = &result[0];
    assert_eq!(*comp_type, riscv_new::WasCompressed::Yes);
    assert_eq!(instruction.mnemonic(), "addi");
    // C.NOP expands to addi x0, x0, 0
    if let riscv_new::Instruction::ADDI { rd, rs1, imm } = instruction {
        assert_eq!(*rd, 0);
        assert_eq!(*rs1, 0);
        assert_eq!(*imm, 0);
    } else {
        panic!("Expected ADDI instruction");
    }
}

#[test]
fn test_c_addi4spn() {
    let decoder = InstructionDecoder::new();
    // C.ADDI4SPN x8, x2, 8 -> 0x0020 (expands to addi x8, x2, 8)
    let bytes = vec![0x20, 0x00]; // 0x0020 in little endian
    let result = decoder.decode_bytes(&bytes).unwrap();
    assert_eq!(result.len(), 1);

    let (instruction, comp_type) = &result[0];
    assert_eq!(*comp_type, riscv_new::WasCompressed::Yes);
    assert_eq!(instruction.mnemonic(), "addi");
    // C.ADDI4SPN expands to addi rd, x2, imm
    if let riscv_new::Instruction::ADDI { rd, rs1, imm } = instruction {
        assert_eq!(*rd, 8); // x8 (compressed reg 0 -> x8)
        assert_eq!(*rs1, 2); // x2 (stack pointer)
        assert_eq!(*imm, 8);
    } else {
        panic!("Expected ADDI instruction");
    }
}

#[test]
fn test_c_lw() {
    let decoder = InstructionDecoder::new();
    // C.LW x8, 0(x12) -> 0x4200 (expands to lw x8, 0(x12))
    let bytes = vec![0x00, 0x42]; // 0x4200 in little endian
    let result = decoder.decode_bytes(&bytes).unwrap();
    assert_eq!(result.len(), 1);

    let (instruction, comp_type) = &result[0];
    assert_eq!(*comp_type, riscv_new::WasCompressed::Yes);
    assert_eq!(instruction.mnemonic(), "lw");
    // C.LW expands to lw rd, offset(rs1)
    if let riscv_new::Instruction::LW { rd, rs1, offset } = instruction {
        assert_eq!(*rd, 8); // x8
        assert_eq!(*rs1, 12); // x12
        assert_eq!(*offset, 0);
    } else {
        panic!("Expected LW instruction");
    }
}

#[test]
fn test_c_ebreak() {
    let decoder = InstructionDecoder::new();
    // C.EBREAK -> 0x9002 (expands to ebreak)
    let bytes = vec![0x02, 0x90]; // 0x9002 in little endian
    let result = decoder.decode_bytes(&bytes).unwrap();
    assert_eq!(result.len(), 1);

    let (instruction, comp_type) = &result[0];
    assert_eq!(*comp_type, riscv_new::WasCompressed::Yes);
    assert_eq!(instruction.mnemonic(), "ebreak");
    // C.EBREAK expands to ebreak
    if let riscv_new::Instruction::EBREAK = instruction {
        // Correct expansion
    } else {
        panic!("Expected EBREAK instruction");
    }
}

#[test]
fn test_decode_bytes_mixed() {
    let decoder = InstructionDecoder::new();

    // Mix of compressed and uncompressed instructions
    let bytes = vec![
        0x01, 0x00, // C.NOP (0x0001)
        0xB3, 0x00, 0x31, 0x00, // ADD x1, x2, x3 (0x003100B3)
        0x85, 0x00, // C.ADDI x1, 1 (0x0085)
    ];

    let result = decoder.decode_bytes(&bytes).unwrap();
    assert_eq!(result.len(), 3);

    // First should be compressed NOP
    let (instr1, comp1) = &result[0];
    assert_eq!(*comp1, riscv_new::WasCompressed::Yes);
    assert_eq!(instr1.mnemonic(), "addi"); // C.NOP expands to addi x0, x0, 0

    // Second should be standard ADD
    let (instr2, comp2) = &result[1];
    assert_eq!(*comp2, riscv_new::WasCompressed::No);
    assert_eq!(instr2.mnemonic(), "add");

    // Third should be compressed ADDI
    let (instr3, comp3) = &result[2];
    assert_eq!(*comp3, riscv_new::WasCompressed::Yes);
    assert_eq!(instr3.mnemonic(), "addi"); // C.ADDI expands to addi
}

#[test]
fn test_rv64_instructions() {
    let decoder = InstructionDecoder::with_target(Target::rv64gc());

    // C.LD should work on RV64
    // C.LD x8, 0(x12) -> 0x6200 (expands to ld x8, 0(x12))
    let bytes = vec![0x00, 0x62]; // 0x6200 in little endian
    let result = decoder.decode_bytes(&bytes).unwrap();
    assert_eq!(result.len(), 1);

    let (instruction, comp_type) = &result[0];
    assert_eq!(*comp_type, riscv_new::WasCompressed::Yes);
    assert_eq!(instruction.mnemonic(), "ld");
    // C.LD expands to ld rd, offset(rs1)
    if let riscv_new::Instruction::LD { rd, rs1, offset } = instruction {
        assert_eq!(*rd, 8); // x8
        assert_eq!(*rs1, 12); // x12
        assert_eq!(*offset, 0);
    } else {
        panic!("Expected LD instruction");
    }
}

#[test]
fn test_rv32_c_ld_fails() {
    let decoder = InstructionDecoder::with_target(Target::rv32imc()); // RV32IMC (has compressed support)

    // C.LD should fail on RV32 during compressed decoding (not alignment)
    let bytes = vec![0x00, 0x62]; // 0x6200 in little endian
    let result = decoder.decode_bytes(&bytes);
    assert!(result.is_err());
}

#[test]
fn test_c_addi_hint_zero_imm() {
    let decoder = InstructionDecoder::new();
    // c.addi x1, 0: funct3=000, imm=0, rd=1, op=01
    let bits: u16 = (0b000 << 13) | (0 << 12) | (1 << 7) | (0 << 2) | 0b01;
    let bytes = bits.to_le_bytes();
    let result = decoder.decode_bytes(&bytes).unwrap();
    assert_eq!(result.len(), 1);
    let (instruction, comp) = &result[0];
    assert_eq!(*comp, riscv_new::WasCompressed::Yes);
    // expands to addi x1, x1, 0 (no-op)
    if let riscv_new::Instruction::ADDI { rd, rs1, imm } = instruction {
        assert_eq!(*rd, 1);
        assert_eq!(*rs1, 1);
        assert_eq!(*imm, 0);
    } else {
        panic!("Expected ADDI expansion for c.addi rd, 0");
    }
}

#[test]
fn test_c_subw_addw_require_rv64() {
    let decoder_rv32 = InstructionDecoder::with_target(Target::rv32imc());
    let decoder_rv64 = InstructionDecoder::with_target(Target::rv64gc());

    // Encode a C.SUBW on RV64: Quadrant 1, funct3=100, funct1=1, funct2_low=00
    // Use rd'=x8 (rd'=0), rs2'=x8 (rs2'=0), rs1'=x8 implicitly via rd'
    // Pattern bits: [15:10]=funct6=1_00_00 (0b100000?)
    // Rather than handcraft exact bits, we'll pick a known encoding seen in other toolchains for demonstration.
    // Here we simulate by taking an existing valid c.subw encoding: 0x9C01 would not be reliable without a full encoder.
    // Instead, we construct via fields: funct3=100 (bits15-13)=0b100, funct1(bit12)=1, funct2_low(bits6-5)=00
    // rd'/rs1'(bits9-7)=000, rs2'(bits4-2)=000, op(bits1-0)=01
    let c_subw_bits: u16 = (0b100 << 13) | (1 << 12) | (0b000 << 7) | (0b00 << 5) | (0b000 << 2) | 0b01;
    let bytes = c_subw_bits.to_le_bytes();

    // RV32 should fail (unsupported on target)
    assert!(decoder_rv32.decode_bytes(&bytes).is_err());
    // RV64 should succeed
    let ok = decoder_rv64.decode_bytes(&bytes);
    assert!(ok.is_ok());

    // Same idea for C.ADDW: funct2_low=01
    let c_addw_bits: u16 = (0b100 << 13) | (1 << 12) | (0b000 << 7) | (0b01 << 5) | (0b000 << 2) | 0b01;
    let bytes2 = c_addw_bits.to_le_bytes();
    assert!(decoder_rv32.decode_bytes(&bytes2).is_err());
    assert!(decoder_rv64.decode_bytes(&bytes2).is_ok());
}

#[test]
fn test_c_shift_imm_shamt5_rv32_reserved() {
    let decoder_rv32 = InstructionDecoder::with_target(Target::rv32imc());
    let decoder_rv64 = InstructionDecoder::with_target(Target::rv64gc());

    // Construct C.SRLI with shamt[5]=1 (bit12=1), funct3=100, funct2=00 (srli)
    // rs1' (bits9-7)=000 (x8). bits6-2 shamt[4:0]=0
    let c_srli_shamt5_bits: u16 = (0b100 << 13) | (1 << 12) | (0b000 << 7) | (0b00 << 10) | (0 << 2) | 0b01;
    let bytes = c_srli_shamt5_bits.to_le_bytes();
    // On RV32 this is reserved
    assert!(decoder_rv32.decode_bytes(&bytes).is_err());
    // On RV64 it's valid
    assert!(decoder_rv64.decode_bytes(&bytes).is_ok());

    // Construct C.SLLI with shamt[5]=1 (bit12=1), funct3=000 (slli)
    // rd (bits11-7)=x1 (non-zero to avoid reserved rd==0), shamt[4:0]=0
    let rd_x1 = 1u16;
    let c_slli_shamt5_bits: u16 = (0b000 << 13) | (1 << 12) | (rd_x1 << 7) | (0 << 2) | 0b10;
    let bytes2 = c_slli_shamt5_bits.to_le_bytes();
    assert!(decoder_rv32.decode_bytes(&bytes2).is_err());
    assert!(decoder_rv64.decode_bytes(&bytes2).is_ok());
}

#[test]
fn test_cl_cs_offset_mappings_lw_sw_nonzero() {
    let decoder = InstructionDecoder::with_target(Target::rv32imc());

    // Build C.LW with non-zero offset: offset bits [6,5,4,3,2] = 1,1,0,1,1 => 0b1101100 = 108
    // Fields (CL-type):
    // funct3=010 (bits15-13), uimm[5:3]=bits12-10, rs1'=bits9-7, uimm[?]: bit6->off[2], bit5->off[6], rd'=bits4-2, op=00
    let funct3 = 0b010u16;
    let off_5_3 = 0b101u16; // offset[5:3]=101
    let off2_bit = 1u16; // offset[2]=1 -> bit6
    let off6_bit = 1u16; // offset[6]=1 -> bit5
    let rs1p = 0b010u16; // x10
    let rdp = 0b011u16; // x11
    let mut bits: u16 = 0;
    bits |= funct3 << 13;
    bits |= off_5_3 << 10;  // bits12:10
    bits |= rs1p << 7;      // bits9:7
    bits |= off2_bit << 6;  // bit6 -> off[2]
    bits |= off6_bit << 5;  // bit5 -> off[6]
    bits |= rdp << 2;       // bits4:2
    bits |= 0b00;           // op

    let bytes = bits.to_le_bytes();
    let res = decoder.decode_bytes(&bytes).unwrap();
    let (instr, comp) = &res[0];
    assert_eq!(*comp, riscv_new::WasCompressed::Yes);
    if let riscv_new::Instruction::LW { rd, rs1, offset } = instr {
        assert_eq!(*rd, 8 + rdp as u8);   // x11
        assert_eq!(*rs1, 8 + rs1p as u8); // x10
        assert_eq!(*offset, 108);
    } else {
        panic!("Expected LW expansion from C.LW");
    }

    // Build matching C.SW with same offset and regs
    let funct3_sw = 0b110u16;
    let rs2p = 0b001u16; // x9 as rs2'
    let mut sw_bits: u16 = 0;
    sw_bits |= funct3_sw << 13;
    sw_bits |= off_5_3 << 10;  // bits12:10
    sw_bits |= rs1p << 7;      // bits9:7
    sw_bits |= off2_bit << 6;  // bit6 -> off[2]
    sw_bits |= off6_bit << 5;  // bit5 -> off[6]
    sw_bits |= rs2p << 2;      // bits4:2
    sw_bits |= 0b00;           // op

    let sw_bytes = sw_bits.to_le_bytes();
    let res2 = decoder.decode_bytes(&sw_bytes).unwrap();
    let (instr2, comp2) = &res2[0];
    assert_eq!(*comp2, riscv_new::WasCompressed::Yes);
    if let riscv_new::Instruction::SW { rs1, rs2, offset } = instr2 {
        assert_eq!(*rs1, 8 + rs1p as u8); // x10
        assert_eq!(*rs2, 8 + rs2p as u8); // x9
        assert_eq!(*offset, 108);
    } else {
        panic!("Expected SW expansion from C.SW");
    }
}

#[test]
fn test_cl_cs_offset_mappings_ld_sd_nonzero() {
    let decoder = InstructionDecoder::with_target(Target::rv64gc());

    // Choose offset for LD/SD: offset[7:6]=11, offset[5:3]=010 => (3<<6)+(2<<3)=192+16=208
    let funct3_ld = 0b011u16; // C.LD
    let off_5_3 = 0b010u16;
    let off_7_6 = 0b11u16;
    let rs1p = 0b100u16; // x12
    let rdp = 0b010u16;  // x10
    let mut ld_bits: u16 = 0;
    ld_bits |= (0b010 << 13) & 0; // placeholder to avoid lint
    ld_bits = 0; // reset
    ld_bits |= funct3_ld << 13;    // bits15:13
    ld_bits |= off_5_3 << 10;      // bits12:10 -> off[5:3]
    ld_bits |= rs1p << 7;          // bits9:7
    ld_bits |= off_7_6 << 5;       // bits6:5 -> off[7:6]
    ld_bits |= rdp << 2;           // bits4:2
    ld_bits |= 0b00;               // op

    let ld_bytes = ld_bits.to_le_bytes();
    let res = decoder.decode_bytes(&ld_bytes).unwrap();
    let (instr, comp) = &res[0];
    assert_eq!(*comp, riscv_new::WasCompressed::Yes);
    if let riscv_new::Instruction::LD { rd, rs1, offset } = instr {
        assert_eq!(*rd, 8 + rdp as u8);   // x10
        assert_eq!(*rs1, 8 + rs1p as u8); // x12
        assert_eq!(*offset, 208);
    } else {
        panic!("Expected LD expansion from C.LD");
    }

    // SD with same offset and regs (rs2' choose 0b011 -> x11)
    let funct3_sd = 0b111u16; // C.SD
    let rs2p = 0b011u16;      // x11
    let mut sd_bits: u16 = 0;
    sd_bits |= funct3_sd << 13;
    sd_bits |= off_5_3 << 10;
    sd_bits |= rs1p << 7;
    sd_bits |= off_7_6 << 5;
    sd_bits |= rs2p << 2;
    sd_bits |= 0b00;

    let sd_bytes = sd_bits.to_le_bytes();
    let res2 = decoder.decode_bytes(&sd_bytes).unwrap();
    let (instr2, comp2) = &res2[0];
    assert_eq!(*comp2, riscv_new::WasCompressed::Yes);
    if let riscv_new::Instruction::SD { rs1, rs2, offset } = instr2 {
        assert_eq!(*rs1, 8 + rs1p as u8);
        assert_eq!(*rs2, 8 + rs2p as u8);
        assert_eq!(*offset, 208);
    } else {
        panic!("Expected SD expansion from C.SD");
    }
}

#[test]
fn test_c_slli_rd_zero_hint() {
    let decoder_rv32 = InstructionDecoder::with_target(Target::rv32imc());
    // c.slli with rd=0 and shamt=1 should be treated as a hint (no-op) and decode/expand
    // bits: funct3=000, shamt[5]=0, rd=0, shamt[4:0]=00001, op=10
    let bits: u16 = (0b000 << 13) | (0 << 12) | (0 << 7) | (0b00001 << 2) | 0b10;
    let bytes = bits.to_le_bytes();
    let res = decoder_rv32.decode_bytes(&bytes).unwrap();
    assert_eq!(res.len(), 1);
    let (instr, comp) = &res[0];
    assert_eq!(*comp, riscv_new::WasCompressed::Yes);
    // expands to slli x0, x0, 1 (no effect)
    if let riscv_new::Instruction::SLLI { rd, rs1, shamt } = instr {
        assert_eq!(*rd, 0);
        assert_eq!(*rs1, 0);
        assert_eq!(*shamt, 1);
    } else {
        panic!("Expected SLLI expansion from c.slli rd=0");
    }
}
