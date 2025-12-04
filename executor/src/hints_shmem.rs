//! HintsShmem is responsible for writting precompile processed hints to shared memory.
//!
//! It implements the HintsSink trait to receive processed hints and write them to shared memory
//! using SharedMemoryWriter instances.

use anyhow::Result;
use asm_runner::SharedMemoryWriter;
use std::sync::Mutex;
use tracing::{debug, warn};
use zisk_hints::HintsSink;

/// HintsShmem struct manages the writing of processed precompile hints to shared memory.
pub struct HintsShmem {
    /// Names of the shared memories to write hints to.
    shmem_names: Vec<String>,

    /// Whether to unlock mapped memory after writing.
    unlock_mapped_memory: bool,

    /// Shared memory writers for writing processed hints.
    shmem_writers: Mutex<Vec<SharedMemoryWriter>>,
}

impl HintsShmem {
    const MAX_PRECOMPILE_SIZE: u64 = 0x10000000; // 256MB

    /// Create a new HintsShmem with the given shared memory names and unlock option.
    ///
    /// # Arguments
    /// * `shmem_names` - A vector of shared memory names to write hints to.
    /// * `unlock_mapped_memory` - Whether to unlock mapped memory after writing.
    ///
    /// # Returns
    /// A new `HintsShmem` instance with uninitialized writers.
    pub fn new(shmem_names: Vec<String>, unlock_mapped_memory: bool) -> Self {
        Self { shmem_names, unlock_mapped_memory, shmem_writers: Mutex::new(Vec::new()) }
    }

    /// Add a shared memory name to the pipeline.
    ///
    /// This method must be called before initialization.
    ///
    /// # Arguments
    /// * `name` - The name of the shared memory to add.
    ///
    /// # Returns
    /// * `Ok(())` - If the name was successfully added or already exists
    /// * `Err` - If writers have already been initialized
    pub fn add_shmem_name(&mut self, name: String) -> Result<()> {
        // Check if the writers have already been initialized
        let shmem_writers = self.shmem_writers.lock().unwrap();
        if !shmem_writers.is_empty() {
            return Err(anyhow::anyhow!(
                "Cannot add shared memory name '{}' after initialization",
                name
            ));
        }

        // Check if the name already exists
        if self.shmem_names.contains(&name) {
            warn!(
                "Shared memory name '{}' already exists in the pipeline. Skipping addition.",
                name
            );
            return Ok(());
        }

        self.shmem_names.push(name);
        Ok(())
    }

    /// Check if the shared memory writers have been initialized.
    fn is_initialized(&self) -> bool {
        let shmem_writers = self.shmem_writers.lock().unwrap();
        !shmem_writers.is_empty()
    }

    /// Initialize the shared memory writers for the pipeline.
    ///
    /// This method creates SharedMemoryWriter instances for each shared memory name.
    /// If writers are already initialized it logs a warning and does nothing.
    fn initialize(&self) {
        let mut shmem_writer = self.shmem_writers.lock().unwrap();

        if !shmem_writer.is_empty() {
            warn!(
                "SharedMemoryWriters for precompile hints is already initialized at '{}'. Skipping",
                self.shmem_names.join(", ")
            );
        } else {
            debug!(
                "Initializing SharedMemoryWriter for precompile hints at '{}'",
                self.shmem_names.join(", ")
            );

            *shmem_writer = self
                .shmem_names
                .iter()
                .map(|name| {
                    SharedMemoryWriter::new(
                        &name,
                        Self::MAX_PRECOMPILE_SIZE as usize,
                        self.unlock_mapped_memory,
                    )
                    .expect("Failed to create SharedMemoryWriter for precompile hints")
                })
                .collect();
        }
    }
}

impl HintsSink for HintsShmem {
    /// Writes processed precompile hints to all shared memory writers.
    ///
    /// # Arguments
    /// * `processed` - A vector of processed precompile hints as u64 values.
    ///
    /// # Returns
    /// * `Ok(())` - If hints were successfully written to all shared memories
    /// * `Err` - If writing to any shared memory fails
    fn submit(&self, processed: Vec<u64>) -> anyhow::Result<()> {
        // TODO! Is it necessary????
        if !self.is_initialized() {
            self.initialize();
        }

        // Input size includes length prefix as u64
        let shmem_input_size = processed.len() + 1;

        let mut full_input = Vec::with_capacity(shmem_input_size);
        // Prefix with length as u64
        full_input.extend_from_slice(&[processed.len() as u64]);
        // Append processed hints
        full_input.extend_from_slice(&processed);

        println!("full_input size: {}", full_input.len());

        let shmem_writers = self.shmem_writers.lock().unwrap();
        for shmem_writer in shmem_writers.iter() {
            shmem_writer.write_input(&full_input)?;
        }

        Ok(())
    }
}
