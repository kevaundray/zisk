use zisk_core::convert_vector_mixed;
use riscv::riscv_interpreter_mixed;

#[test]
fn test_compressed_instruction_parsing() {
    println!("Testing Zisk compressed instruction support...");
    
    // Test data: Mix of compressed and uncompressed instructions
    // This represents a simple program with both types of instructions
    let test_data = vec![
        // C.ADDI x1, x1, 1 (compressed) - 0x0085 (little endian: 0x85, 0x00)
        0x85, 0x00,
        // C.NOP (compressed) - 0x0001 (little endian: 0x01, 0x00)  
        0x01, 0x00,
        // ADDI x2, x1, 42 (uncompressed) - 0x02a08113 (little endian)
        0x13, 0x81, 0xa0, 0x02,
        // C.MV x3, x2 (compressed) - 0x81aa (little endian: 0xaa, 0x81)
        0xaa, 0x81,
    ];
    
    let base_addr = 0x1000;
    
    // Test our mixed instruction parser
    println!("Parsing mixed instruction stream...");
    let instruction_words = convert_vector_mixed(&test_data, base_addr);
    
    println!("Found {} instruction words:", instruction_words.len());
    for (i, inst_word) in instruction_words.iter().enumerate() {
        println!("  {}: addr=0x{:x}, instruction=0x{:x}, compressed={}",
                 i, inst_word.addr, inst_word.instruction, inst_word.is_compressed);
    }
    
    // Test our RISC-V instruction interpreter
    println!("\nDecoding instructions...");
    let riscv_instructions = riscv_interpreter_mixed(&instruction_words);
    
    println!("Decoded {} RISC-V instructions:", riscv_instructions.len());
    for (i, inst) in riscv_instructions.iter().enumerate() {
        println!("  {}: {} ({})", i, inst.inst, 
                 if inst.is_compressed { "compressed" } else { "uncompressed" });
        println!("      addr=0x{:x}, rd={}, rs1={}, rs2={}, imm={}",
                 inst.addr, inst.rd, inst.rs1, inst.rs2, inst.imm);
    }
    
    // Verify expected results
    assert_eq!(instruction_words.len(), 4, "Should have 4 instruction words");
    assert_eq!(riscv_instructions.len(), 4, "Should have decoded 4 instructions");
    
    // Check the first instruction (C.ADDI)
    assert!(riscv_instructions[0].is_compressed, "First instruction should be compressed");
    assert_eq!(riscv_instructions[0].inst, "addi", "First instruction should be ADDI");
    assert_eq!(riscv_instructions[0].addr, 0x1000, "First instruction address should be 0x1000");
    
    // Check the third instruction (uncompressed ADDI)
    assert!(!riscv_instructions[2].is_compressed, "Third instruction should be uncompressed");
    assert_eq!(riscv_instructions[2].inst, "addi", "Third instruction should be ADDI");
    assert_eq!(riscv_instructions[2].addr, 0x1004, "Third instruction address should be 0x1004");
    
    println!("\n✅ All tests passed! Compressed instruction support is working correctly.");
}

#[test]
fn test_pure_compressed_instructions() {
    // Test a series of common compressed instructions
    let test_data = vec![
        // C.LI x1, 5 (load immediate) - 0x4085
        0x85, 0x40,
        // C.ADDI x1, x1, 3 - 0x008d
        0x8d, 0x00,
        // C.LW x2, 0(x1) (load word) - 0x4100
        0x00, 0x41,
    ];
    
    let base_addr = 0x2000;
    let instruction_words = convert_vector_mixed(&test_data, base_addr);
    let riscv_instructions = riscv_interpreter_mixed(&instruction_words);
    
    // All should be compressed
    assert_eq!(instruction_words.len(), 3);
    for inst_word in &instruction_words {
        assert!(inst_word.is_compressed);
    }
    
    // Check instruction types
    assert_eq!(riscv_instructions[0].inst, "addi"); // C.LI maps to ADDI with rs1=x0
    assert_eq!(riscv_instructions[1].inst, "addi"); // C.ADDI maps to ADDI
    assert_eq!(riscv_instructions[2].inst, "lw");   // C.LW maps to LW
}