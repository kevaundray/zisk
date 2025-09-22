# ZisK zkVM - Deep Technical Documentation

## Table of Contents
1. [Project Overview](#project-overview)
2. [Architecture Overview](#architecture-overview)
3. [Core Components](#core-components)
4. [State Machine Architecture](#state-machine-architecture)
5. [Execution Flow](#execution-flow)
6. [Memory Model](#memory-model)
7. [PIL Integration](#pil-integration)
8. [Performance Optimizations](#performance-optimizations)
9. [Development Workflow](#development-workflow)
10. [API Reference](#api-reference)
11. [Troubleshooting](#troubleshooting)

---

## Project Overview

**ZisK** is a high-performance zero-knowledge virtual machine (zkVM) developed by Polygon that enables trustless, verifiable computation. The project allows developers to generate and verify proofs for arbitrary program execution efficiently, with Rust as the primary language for writing provable programs.

### Key Features
- **High-performance architecture** optimized for low-latency proof generation
- **RISC-V compatibility** - executes standard RISC-V ELF binaries
- **Modular design** with specialized state machines for different operations
- **Parallel processing** capabilities for witness computation
- **Developer-friendly** tooling with comprehensive CLI interface
- **Open source** with Apache-2.0 or MIT dual licensing

### Project Status
⚠️ **Active Development**: The software is currently under development and has not been audited. Do not use in production environments.

---

## Architecture Overview

ZisK follows a sophisticated multi-layered architecture that separates concerns between execution, witness generation, and proof creation:

```
┌─────────────────┐    ┌─────────────────┐    ┌─────────────────┐
│   RISC-V ELF    │ -> │  ZisK ROM       │ -> │  Execution      │
│   Binary        │    │  Conversion     │    │  Traces         │
└─────────────────┘    └─────────────────┘    └─────────────────┘
                                                       │
┌─────────────────┐    ┌─────────────────┐    ┌─────────────────┐
│   ZK Proof      │ <- │  Witness        │ <- │  State Machine  │
│   Generation    │    │  Computation    │    │  Processing     │
└─────────────────┘    └─────────────────┘    └─────────────────┘
```

### Core Architectural Principles

1. **Modular State Machines**: Complex operations are delegated to specialized state machines
2. **Parallel Processing**: Multi-threaded execution with configurable parallelization
3. **Memory Efficiency**: Sophisticated trace buffer management and memory pooling
4. **PIL Integration**: Uses Polynomial Identity Language for constraint definitions

---

## Core Components

### 1. Core Module (`/core/src/`)

The foundational layer containing ZisK instruction definitions and RISC-V integration.

**Key Files:**
- `lib.rs` - Core data structures and fundamental operations
- `zisk_inst.rs` - ZisK instruction format and execution logic
- `riscv2zisk.rs` - RISC-V to ZisK transpilation

**ZisK Instruction Format:**
```rust
pub struct ZiskInst {
    pub paddr: u64,              // Program address
    pub store_ra: bool,          // Store return address flag
    pub a_src: u64,              // Source for operand a
    pub b_src: u64,              // Source for operand b
    pub op: u8,                  // Operation code
    pub func: fn(&mut InstContext) -> (), // Operation function
    pub op_type: ZiskOperationType, // Internal/External classification
}
```

### 2. Emulator (`/emulator/src/`)

High-performance execution environment with dual backends (Rust and Assembly).

**Key Features:**
- **Multi-threaded execution** with configurable thread pools (default: 16 threads)
- **Segmented processing** with configurable chunk sizes
- **Assembly integration** for performance-critical paths
- **Trace generation** with minimal overhead

**Execution Phases:**
1. **EXECUTE**: Fast initial execution generating minimal traces
2. **COUNT**: Parallel processing to collect operation metrics
3. **EXPAND**: Full witness computation with detailed traces

### 3. State Machines (`/state-machines/`)

Specialized processing units for different operation types:

#### Main State Machine (`/state-machines/main/`)
- **Primary execution engine** processing ZisK instructions sequentially
- **Memory management** with register operations and linked list integrity
- **Segmentation support** for parallel processing
- **Continuation handling** for segment chaining

#### Secondary State Machines:
- **Arithmetic** (`/state-machines/arith/`) - Complex mathematical operations
- **Binary** (`/state-machines/binary/`) - Bitwise and logical operations  
- **Memory** (`/state-machines/mem/`) - Memory access with alignment handling
- **ROM** (`/state-machines/rom/`) - Program storage and instruction fetching

### 4. Executor (`/executor/src/`)

Orchestrates the entire computation pipeline from input to proof generation.

**Key Responsibilities:**
- **Phase coordination** across all execution stages
- **Resource management** and thread pool allocation
- **State machine coordination** and communication
- **Performance monitoring** and statistics collection

### 5. Witness Computation (`/witness-computation/src/`)

Integrates all components for cryptographic witness generation.

**Integration Flow:**
```rust
impl<F: PrimeField64> WitnessLibrary<F> for WitnessLib<F> {
    fn register_witness(&mut self, wcm: &WitnessManager<F>) {
        // Initialize state machines
        let binary_sm = BinarySM::new(std.clone());
        let arith_sm = ArithSM::new(std.clone());
        let mem_sm = Mem::new(std.clone());
        
        // Create and register executor
        let executor = ZiskExecutor::new(/* parameters */);
        wcm.register_component(executor.clone());
    }
}
```

### 6. CLI Interface (`/cli/src/`)

Comprehensive command-line interface supporting:
- Program building and execution
- Proof generation and verification  
- Performance analysis and debugging
- Server mode for distributed execution

---

## State Machine Architecture

### Hierarchical Design

ZisK implements a hierarchical state machine architecture where the main state machine delegates complex operations to specialized secondary state machines through a bus-based communication protocol.

### Communication Protocol

**Bus-based Architecture:**
- Standardized communication format between state machines
- Parallel processing capabilities
- Automated result integration and proof aggregation

**Operation Classification:**
- **Internal Operations**: Handled directly by the main state machine
- **External Operations**: Delegated to appropriate secondary state machines

### State Machine Specialization

Each state machine is optimized for specific operation types:

1. **Main SM**: Core instruction processing and control flow
2. **Arith SM**: Field arithmetic, modular operations
3. **Binary SM**: Bitwise operations, shifts, logical operations
4. **Mem SM**: Memory access, alignment handling, caching
5. **ROM SM**: Program loading, instruction fetching

---

## Execution Flow

### 1. Program Preparation
```
RISC-V ELF → ZisK ROM Conversion → Instruction Analysis
```

### 2. Emulation Phase
```
ROM Loading → Multi-threaded Execution → Trace Generation
```

### 3. State Machine Processing
```
Operation Classification → Delegation → Parallel Processing
```

### 4. Witness Generation
```
Trace Collection → Constraint Evaluation → Witness Computation
```

### 5. Proof Generation
```
Witness Integration → PIL Processing → ZK Proof Output
```

---

## Memory Model

### Address Space Layout

- **Register Space**: `REG_BASE_ADDR = 0xA000_0000` - Dedicated register operations
- **Memory Space**: General-purpose memory with alignment support
- **Stack Space**: Program stack with automatic growth management

### Memory Operations

**Alignment Handling:**
- Support for unaligned memory operations
- Specialized processing for byte, word, and double-word accesses
- Cache-efficient chunked processing

**Timestamping:**
- Fine-grained memory step tracking (4 mem_steps per main_step)
- Ordered access pattern maintenance
- Integrity verification through linked list structures

---

## PIL Integration

### Polynomial Identity Language (PIL)

PIL defines the constraint system for zkSNARK circuits. Key components:

**Core Registers:**
```pil
airtemplate Main(int N = 2**21, int RC = 2, int stack_enabled = 0) {
    col witness a[RC];           // Input operand A
    col witness b[RC];           // Input operand B  
    col witness c[RC];           // Result operand C
    col witness flag;            // Conditional flag
    col witness pc;              // Program counter
}
```

**Constraint Types:**
- **Bus Operations**: Inter-state-machine communication
- **Range Checks**: Memory access validation
- **Continuations**: Segment chaining integrity
- **Lookup Operations**: Efficient table-based constraints

---

## Performance Optimizations

### 1. Assembly Integration

**Native Code Generation:**
- Direct assembly code generation for critical execution paths
- SIMD instruction utilization where applicable
- Register allocation optimization

### 2. Parallel Processing

**Segmented Execution:**
- Configurable chunk sizes for optimal parallelization
- Dynamic workload distribution across thread pools
- Memory bandwidth optimization through distributed processing

### 3. Data Structure Optimization

**Memory Management:**
- Trace compression for efficient storage
- Memory pooling and reusable buffer allocation
- Cache-conscious data structure layout

**Key Optimizations:**
- **Minimal Trace Mode**: Reduced memory overhead during initial execution
- **Batch Processing**: Grouped operations for better cache utilization
- **Lazy Evaluation**: Deferred computation of expensive operations

---

## Development Workflow

### 1. Environment Setup

**Prerequisites:**
- Rust toolchain (specified in `rust-toolchain.toml`)
- RISC-V toolchain for cross-compilation
- Git with LFS support for large test files

**Installation:**
```bash
# Clone repository
git clone https://github.com/0xPolygonHermez/zisk.git
cd zisk

# Build project
cargo build --release

# Run tests
cargo test
```

### 2. Program Development

**Writing ZisK Programs:**
```rust
#![no_std]
#![no_main]

use ziskclib::*;

#[no_mangle]
pub extern "C" fn main() {
    // Your provable program logic here
}
```

**Build Process:**
```bash
# Build for ZisK
cargo zisk build

# Execute with proof generation
cargo zisk prove --input input.json
```

### 3. Testing and Debugging

**Test Suite Organization:**
- Unit tests for individual components
- Integration tests for end-to-end workflows
- Regression tests with ELF binaries (`/elf-regressions/`)

**Debugging Tools:**
- Trace analysis utilities
- Performance profiling integration
- Memory usage monitoring

---

## API Reference

### Command Line Interface

**Basic Commands:**
```bash
# Build a program
cargo zisk build [OPTIONS] <PATH>

# Execute and generate proof
cargo zisk prove [OPTIONS] <ELF_PATH>

# Verify a proof
cargo zisk verify [OPTIONS] <PROOF_PATH>

# Start server mode
cargo zisk server [OPTIONS]
```

**Configuration Options:**
- `--threads <N>`: Set number of execution threads
- `--chunk-size <SIZE>`: Configure segmentation size
- `--output <FORMAT>`: Specify output format (json, binary)
- `--debug`: Enable debug output and tracing

### Library Integration

**Rust Integration:**
```rust
use zisk_core::*;
use ziskemu::*;

// Create emulator instance
let mut emulator = Emulator::new();

// Load program
emulator.load_rom(&rom_data);

// Execute with inputs
let traces = emulator.execute(&input_data);

// Generate proof
let proof = generate_proof(&traces);
```

---

## Project Structure

### Workspace Organization

The project is organized as a Cargo workspace with the following structure:

```
zisk/
├── cli/                    # Command-line interface
├── core/                   # Core ZisK functionality
├── emulator/               # Execution engine
├── executor/               # Orchestration layer
├── state-machines/         # Specialized processing units
│   ├── main/              # Main state machine
│   ├── arith/             # Arithmetic operations
│   ├── binary/            # Binary operations
│   ├── mem/               # Memory operations
│   └── rom/               # ROM operations
├── witness-computation/    # Proof witness generation
├── precompiles/           # Optimized function implementations
├── tools/                 # Development utilities
├── book/                  # Documentation source
├── elf-regressions/       # Test programs
└── lib-c/                 # C library bindings
```

### Key Configuration Files

- **Cargo.toml**: Workspace configuration and dependencies
- **book.toml**: Documentation book configuration
- **rust-toolchain.toml**: Rust version specification
- **rustfmt.toml**: Code formatting rules
- **clippy.toml**: Linting configuration

---

## Troubleshooting

### Common Issues

**1. Build Failures**
- Ensure correct Rust toolchain version
- Check RISC-V toolchain installation
- Verify all git submodules are initialized

**2. Memory Issues**
- Reduce chunk size for large programs
- Increase system memory limits
- Use minimal trace mode for initial testing

**3. Performance Problems**
- Adjust thread count for your system
- Enable assembly optimizations
- Use release builds for benchmarking

### Debug Information

**Environment Variables:**
```bash
export RUST_LOG=debug          # Enable debug logging
export ZISK_THREADS=8          # Override thread count
export ZISK_CHUNK_SIZE=1024    # Set chunk size
```

**Diagnostic Commands:**
```bash
# System information
cargo zisk info

# Trace analysis
cargo zisk trace --analyze <ELF_PATH>

# Performance profiling
cargo zisk bench <ELF_PATH>
```

---

## External Dependencies

### Core Dependencies

- **proofman**: PIL2-based proving system integration
- **fields**: Finite field arithmetic (Goldilocks field)
- **ark-ff**: Arkworks finite field implementations
- **rayon**: Data parallelism framework
- **serde**: Serialization framework

### Development Dependencies

- **tracing**: Structured logging and diagnostics
- **clap**: Command-line argument parsing
- **anyhow**: Error handling
- **itertools**: Iterator utilities

---

## Contributing

### Code Style

The project follows standard Rust conventions with additional requirements:

- Use `rustfmt` for code formatting
- Pass `clippy` linting checks
- Include comprehensive tests for new features
- Document public APIs with rustdoc

### Testing Requirements

- Unit tests for individual components
- Integration tests for cross-component functionality
- Regression tests for critical bug fixes
- Performance benchmarks for optimization changes

### Pull Request Process

1. Fork the repository and create a feature branch
2. Implement changes with appropriate tests
3. Ensure all CI checks pass
4. Submit pull request with detailed description
5. Address review feedback and iterate

---

## License

This project is dual-licensed under:

- **Apache License, Version 2.0** ([LICENSE-APACHE](LICENSE-APACHE))
- **MIT License** ([LICENSE-MIT](LICENSE-MIT))

You may choose either license at your discretion.

---

## Acknowledgements

ZisK is built upon foundational work from:

- **Polygon zkEVM Team**: Zero-knowledge proving system expertise
- **Plonky3**: Advanced constraint system implementations  
- **RISC-V Community**: Robust virtual machine architecture
- **Open Source Cryptography Community**: Ongoing research and development

Special thanks to all contributors who have helped develop, refine, and improve ZisK!

---

*This documentation was generated based on ZisK version 0.12.0. For the latest updates, please refer to the official repository and documentation.*