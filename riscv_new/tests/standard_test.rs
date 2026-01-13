use riscv_new::{InstructionDecoder, Target};

#[test]
fn test_slli_invalid_upper_bits_rv32() {
    let decoder = InstructionDecoder::with_target(Target::rv32imc());
    
    // SLLI x1, x2, 1 with invalid upper bits (funct7 = 0b001_0000 instead of 0b000_0000)
    // Format: imm[11:5]=0010000, rs1=x2=00010, funct3=001, rd=x1=00001, opcode=0010011
    let invalid_bits: u32 = 
        (0b001_0000 << 25) |  // imm[11:5] - invalid (should be 0)
        (2 << 20) |           // rs1 = x2
        (1 << 15) |           // rs1 continued
        (0b001 << 12) |       // funct3 = SLLI
        (1 << 7) |            // rd = x1
        0b0010011;            // opcode = OP-IMM
    
    let bytes = invalid_bits.to_le_bytes();
    let result = decoder.decode_bytes(&bytes);
    assert!(result.is_err()); // Should fail due to invalid upper bits
}

#[test]
fn test_slli_invalid_upper_bits_rv64() {
    let decoder = InstructionDecoder::with_target(Target::rv64gc());
    
    // SLLI x1, x2, 1 with invalid upper bits (imm[11:6] = 0b10_0000 instead of 0b00_0000)
    // Format: imm[11:6]=100000, imm[5]=0 (shamt[5]), rs1=x2, funct3=001, rd=x1, opcode=0010011
    let invalid_bits: u32 = 
        (0b100_000_0 << 25) | // imm[11:5] where imm[11:6]=100000 (invalid), imm[5]=0
        (2 << 20) |           // rs1 = x2
        (0b001 << 12) |       // funct3 = SLLI
        (1 << 7) |            // rd = x1
        0b0010011;            // opcode = OP-IMM
    
    let bytes = invalid_bits.to_le_bytes();
    let result = decoder.decode_bytes(&bytes);
    assert!(result.is_err()); // Should fail due to invalid upper bits
}

#[test]
fn test_slli_valid_upper_bits_rv32() {
    let decoder = InstructionDecoder::with_target(Target::rv32imc());
    
    // Valid SLLI x1, x2, 5 (shamt=5, upper bits = 0)
    let valid_bits: u32 = 
        (5 << 20) |           // imm[11:0] = shamt=5 (bits 24:20)
        (2 << 15) |           // rs1 = x2 (bits 19:15)
        (0b001 << 12) |       // funct3 = SLLI
        (1 << 7) |            // rd = x1
        0b0010011;            // opcode = OP-IMM
    
    let bytes = valid_bits.to_le_bytes();
    let result = decoder.decode_bytes(&bytes).unwrap();
    assert_eq!(result.len(), 1);
    
    let (instruction, _) = &result[0];
    if let riscv_new::Instruction::SLLI { rd, rs1, shamt } = instruction {
        assert_eq!(*rd, 1);
        assert_eq!(*rs1, 2);
        assert_eq!(*shamt, 5);
    } else {
        panic!("Expected SLLI instruction");
    }
}

#[test]
fn test_slli_valid_upper_bits_rv64() {
    let decoder = InstructionDecoder::with_target(Target::rv64gc());
    
    // Valid SLLI x1, x2, 33 (shamt=33, requires 6 bits, upper 6 bits = 0)
    let shamt = 33u32;
    let valid_bits_correct: u32 = 
        (shamt << 20) |       // imm[11:0] contains shamt in lower 6 bits for RV64
        (2 << 15) |           // rs1 = x2 (bits 19:15)
        (0b001 << 12) |       // funct3 = SLLI
        (1 << 7) |            // rd = x1
        0b0010011;            // opcode = OP-IMM
    
    let bytes = valid_bits_correct.to_le_bytes();
    let result = decoder.decode_bytes(&bytes).unwrap();
    assert_eq!(result.len(), 1);
    
    let (instruction, _) = &result[0];
    if let riscv_new::Instruction::SLLI { rd, rs1, shamt } = instruction {
        assert_eq!(*rd, 1);
        assert_eq!(*rs1, 2);
        assert_eq!(*shamt, 33);
    } else {
        panic!("Expected SLLI instruction");
    }
}

#[test]
fn test_srli_srai_invalid_upper_bits_rv32() {
    let decoder = InstructionDecoder::with_target(Target::rv32imc());
    
    // Invalid SRLI with wrong upper bits (should be 0b000_0000, using 0b001_0000)
    let invalid_srli_bits: u32 = 
        (0b001_0101 << 25) |  // imm[11:5] = 0010101 (invalid upper, shamt=5)
        (2 << 20) |           // rs1 = x2
        (0b101 << 12) |       // funct3 = SRLI/SRAI
        (1 << 7) |            // rd = x1
        0b0010011;            // opcode = OP-IMM
    
    let bytes = invalid_srli_bits.to_le_bytes();
    let result = decoder.decode_bytes(&bytes);
    assert!(result.is_err());
    
    // Invalid SRAI with wrong upper bits (should be 0b010_0000, using 0b011_0000)
    let invalid_srai_bits: u32 = 
        (0b011_0101 << 25) |  // imm[11:5] = 0110101 (invalid)
        (2 << 20) |           // rs1 = x2
        (0b101 << 12) |       // funct3 = SRLI/SRAI
        (1 << 7) |            // rd = x1
        0b0010011;            // opcode = OP-IMM
    
    let bytes2 = invalid_srai_bits.to_le_bytes();
    let result2 = decoder.decode_bytes(&bytes2);
    assert!(result2.is_err());
}

#[test]
fn test_srli_srai_invalid_upper_bits_rv64() {
    let decoder = InstructionDecoder::with_target(Target::rv64gc());
    
    // Invalid SRLI with wrong upper 6 bits (should be 0b00_0000, using 0b10_0000)
    let invalid_srli_bits: u32 = 
        (0b100_000_1 << 25) | // imm[11:5] where upper 6 bits = 100000 (invalid)
        (2 << 20) |           // rs1 = x2
        (0b101 << 12) |       // funct3 = SRLI/SRAI
        (1 << 7) |            // rd = x1
        0b0010011;            // opcode = OP-IMM
    
    let bytes = invalid_srli_bits.to_le_bytes();
    let result = decoder.decode_bytes(&bytes);
    assert!(result.is_err());
    
    // Invalid SRAI with wrong upper 6 bits (should be 0b01_0000, using 0b11_0000)
    let invalid_srai_bits: u32 = 
        (0b110_000_1 << 25) | // imm[11:5] where upper 6 bits = 110000 (invalid)
        (2 << 20) |           // rs1 = x2
        (0b101 << 12) |       // funct3 = SRLI/SRAI
        (1 << 7) |            // rd = x1
        0b0010011;            // opcode = OP-IMM
    
    let bytes2 = invalid_srai_bits.to_le_bytes();
    let result2 = decoder.decode_bytes(&bytes2);
    assert!(result2.is_err());
}

#[test]
fn test_srli_srai_valid_patterns() {
    let decoder_rv32 = InstructionDecoder::with_target(Target::rv32imc());
    let decoder_rv64 = InstructionDecoder::with_target(Target::rv64gc());
    
    // Valid SRLI RV32: funct7=0000000, shamt=5
    let valid_srli_rv32: u32 = 
        (5 << 20) |           // imm[11:0] = shamt=5 (bits 24:20), funct7=0 (bits 31:25)
        (2 << 15) |           // rs1 = x2
        (0b101 << 12) |       // funct3 = SRLI/SRAI
        (1 << 7) |            // rd = x1
        0b0010011;            // opcode = OP-IMM
    
    let bytes = valid_srli_rv32.to_le_bytes();
    let result = decoder_rv32.decode_bytes(&bytes).unwrap();
    if let riscv_new::Instruction::SRLI { rd, rs1, shamt } = &result[0].0 {
        assert_eq!(*rd, 1);
        assert_eq!(*rs1, 2);
        assert_eq!(*shamt, 5);
    } else {
        panic!("Expected SRLI instruction");
    }
    
    // Valid SRAI RV32: funct7=0100000, shamt=5
    let valid_srai_rv32: u32 = 
        (0b0100000 << 25) |   // funct7=0100000 (SRAI pattern)
        (5 << 20) |           // shamt=5 (bits 24:20)
        (2 << 15) |           // rs1 = x2
        (0b101 << 12) |       // funct3 = SRLI/SRAI
        (1 << 7) |            // rd = x1
        0b0010011;            // opcode = OP-IMM
    
    let bytes2 = valid_srai_rv32.to_le_bytes();
    let result2 = decoder_rv32.decode_bytes(&bytes2).unwrap();
    if let riscv_new::Instruction::SRAI { rd, rs1, shamt } = &result2[0].0 {
        assert_eq!(*rd, 1);
        assert_eq!(*rs1, 2);
        assert_eq!(*shamt, 5);
    } else {
        panic!("Expected SRAI instruction");
    }
    
    // Valid SRLI RV64: upper 6 bits = 000000, shamt=33
    let valid_srli_rv64: u32 = 
        (33 << 20) |          // shamt=33 in bits[25:20], funct7[31:26]=000000
        (2 << 15) |           // rs1 = x2
        (0b101 << 12) |       // funct3 = SRLI/SRAI
        (1 << 7) |            // rd = x1
        0b0010011;            // opcode = OP-IMM
    
    let bytes3 = valid_srli_rv64.to_le_bytes();
    let result3 = decoder_rv64.decode_bytes(&bytes3).unwrap();
    if let riscv_new::Instruction::SRLI { rd, rs1, shamt } = &result3[0].0 {
        assert_eq!(*rd, 1);
        assert_eq!(*rs1, 2);
        assert_eq!(*shamt, 33);
    } else {
        panic!("Expected SRLI instruction");
    }
    
    // Valid SRAI RV64: upper 6 bits = 010000, shamt=33
    let valid_srai_rv64_correct: u32 = 
        (0b0100000 << 25) |    // funct7[31:25] = 0100000 for SRAI
        (33 << 20) |           // shamt=33 in bits[25:20]
        (2 << 15) |            // rs1 = x2
        (0b101 << 12) |        // funct3 = SRLI/SRAI
        (1 << 7) |             // rd = x1
        0b0010011;             // opcode = OP-IMM
    
    let bytes4 = valid_srai_rv64_correct.to_le_bytes();
    let result4 = decoder_rv64.decode_bytes(&bytes4).unwrap();
    if let riscv_new::Instruction::SRAI { rd, rs1, shamt } = &result4[0].0 {
        assert_eq!(*rd, 1);
        assert_eq!(*rs1, 2);
        assert_eq!(*shamt, 33);
    } else {
        panic!("Expected SRAI instruction");
    }
}