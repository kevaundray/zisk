//! Precompile Hints Processor
//!
//! This module provides functionality for parsing and processing precompile hints
//! that are received as a stream of `u64` values. Hints are used to provide preprocessed
//! data to precompile operations in the ZisK zkVM.
//!
//! # Hint Format
//!
//! Each hint consists of:
//! - A **header** (`u64`): Contains the hint type (upper 32 bits) and data length (lower 32 bits)
//! - **Data** (`[u64; length]`): The hint payload, where `length` is specified in the header
//!
//! ```text
//! ┌────────────────────────────────────────────────────────────────┐
//! │                         Header (u64)                           │
//! ├────────────────────────────────┬───────────────────────────────┤
//! │      Hint Code (32 bits)       │       Length (32 bits)        │
//! ├────────────────────────────────┴───────────────────────────────┤
//! │                      Data[0] (u64)                             │
//! ├────────────────────────────────────────────────────────────────┤
//! │                      Data[1] (u64)                             │
//! ├────────────────────────────────────────────────────────────────┤
//! │                         ...                                    │
//! ├────────────────────────────────────────────────────────────────┤
//! │                      Data[length-1] (u64)                      │
//! └────────────────────────────────────────────────────────────────┘
//! 
//! - Hint Code — Control code or Data Hint Type
//! - Length — Number of following u64 data words
//!
//! ## Hint Type Layout
//!
//! ### Control codes
//! 
//! The following control codes are defined:
//! - `0x00` (START): Reset processor state and global sequence.
//! - `0x01` (END): Wait until completion of all pending hints.
//! - `0x02` (CANCEL): Cancel current stream and stop processing further hints. 
//! - `0x03` (ERROR): Indicate an error has occurred; stop processing further hints.
//! 
//! Control codes are for control only and do not have any associated data (Length should be zero).
//! 
//! ### Data Hint Types:
//! - `0x04` (`HINTS_TYPE_RESULT`): Pass-through data
//! - `0x05` (`HINTS_TYPE_ECRECOVER`): ECRECOVER inputs (currently returns empty)
//! ```

use anyhow::Result;
use rayon::{ThreadPool, ThreadPoolBuilder};
use std::collections::VecDeque;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::{Arc, Condvar, Mutex};

/// Hint type indicating that the data is already the precomputed result.
///
/// When a hint has this type, the processor simply passes through the data
/// without any additional computation.
pub const HINTS_TYPE_RESULT: u32 = 1;

/// Hint type indicating that the data contains inputs for the ecrecover precompile.
pub const HINTS_TYPE_ECRECOVER: u32 = 2;

/// Stream control is encoded in the high byte (bits 31..24) of `hint_type`.
/// Base type is the lower 24 bits (bits 23..0).
const STREAM_CTRL_MASK: u32 = 0xFF00_0000;
const STREAM_BASE_MASK: u32 = 0x00FF_FFFF;
const STREAM_CTRL_SHIFT: u32 = 24;

const STREAM_CTRL_NONE: u32 = 0x00;
const STREAM_CTRL_START: u32 = 0x01; // reset stream state
const STREAM_CTRL_END: u32 = 0x02; // wait until completion
const STREAM_CTRL_CANCEL: u32 = 0x03; // cancel processing
const STREAM_CTRL_ERROR: u32 = 0x04; // signal error

/// Represents a single precompile hint parsed from a `u64` slice.
///
/// A hint consists of a type identifier and associated data. The hint type
/// determines how the data should be processed by the [`PrecompileHintsProcessor`].
pub struct PrecompileHint {
    /// The type of hint, determining how the data should be processed.
    hint_type: u32,
    /// The hint payload data.
    data: Vec<u64>,
}

impl std::fmt::Debug for PrecompileHint {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("PrecompileHint")
            .field("hint_type", &self.hint_type)
            .field("data", &self.data)
            .finish()
    }
}

impl PrecompileHint {
    /// Parses a [`PrecompileHint`] from a slice of `u64` values at the given index.
    ///
    /// # Arguments
    ///
    /// * `slice` - The source slice containing concatenated hints
    /// * `idx` - The index where the hint header starts
    ///
    /// # Returns
    ///
    /// * `Ok(PrecompileHint)` - Successfully parsed hint
    /// * `Err` - If the slice is too short or the index is out of bounds
    #[inline(always)]
    fn from_u64_slice(slice: &[u64], idx: usize) -> Result<Self> {
        if slice.is_empty() || idx >= slice.len() {
            return Err(anyhow::anyhow!("Slice too short to contain a hint"));
        }

        let header = slice[idx];
        let hint_type = (header >> 32) as u32;
        let length = (header & 0xFFFFFFFF) as u32;

        if slice.len() < idx + length as usize + 1 {
            return Err(anyhow::anyhow!(
                "Slice too short for hint data: expected {}, got {}",
                length,
                slice.len() - idx - 1
            ));
        }

        let data = slice[idx + 1..idx + length as usize + 1].to_vec();

        Ok(PrecompileHint { hint_type, data })
    }
}

/// Shared state for the reorder buffer used by `process_hints_2`.
///
/// This structure maintains a global sequence counter and a VecDeque that
/// holds processed results in order, allowing out-of-order completion while
/// ensuring in-order output.
struct ReorderBuffer {
    /// The reorder buffer: None = pending, Some(Ok(...)) = ready, Some(Err(...)) = error
    buffer: VecDeque<Option<Result<Vec<u64>>>>,
    /// Sequence ID of buffer[0] (the next result to drain/print)
    base_seq: usize,
}

/// Shared state across multiple calls to `process_hints_2`.
struct SharedState {
    /// The reorder buffer protected by a mutex
    reorder: Mutex<ReorderBuffer>,
    /// Condvar to signal when buffer becomes empty or error occurs
    buffer_empty: Condvar,
    /// Global sequence counter for assigning seq_ids to hints
    next_seq: AtomicUsize,
    /// Flag to signal that an error occurred and processing should stop
    has_error: AtomicBool,
    /// Generation counter to detect stale workers after reset
    generation: AtomicUsize,
}

impl SharedState {
    fn new() -> Self {
        Self {
            reorder: Mutex::new(ReorderBuffer { buffer: VecDeque::new(), base_seq: 0 }),
            buffer_empty: Condvar::new(),
            next_seq: AtomicUsize::new(0),
            has_error: AtomicBool::new(false),
            generation: AtomicUsize::new(0),
        }
    }
}

/// Processor for precompile hints that supports parallel execution.
///
/// This struct provides methods to parse and process a stream of concatenated
/// hints, using a dedicated Rayon thread pool for parallel processing while
/// preserving the original order of results.
pub struct PrecompileHintsProcessor {
    /// The thread pool used for parallel hint processing.
    pool: ThreadPool,
    /// Shared state for the reorder buffer (used by process_hints_2)
    shared: Arc<SharedState>,
}

impl PrecompileHintsProcessor {
    const NUM_THREADS: usize = 32;

    /// Creates a new processor with the default number of threads.
    ///
    /// The default is the number of available CPU cores.
    ///
    /// # Returns
    ///
    /// * `Ok(PrecompileHintsProcessor)` - The configured processor
    /// * `Err` - If the thread pool fails to initialize
    pub fn new() -> Result<Self> {
        Self::with_num_threads(Self::NUM_THREADS)
    }

    /// Creates a new processor with the specified number of threads.
    ///
    /// # Arguments
    ///
    /// * `num_threads` - The number of worker threads in the pool
    ///
    /// # Returns
    ///
    /// * `Ok(PrecompileHintsProcessor)` - The configured processor
    /// * `Err` - If the thread pool fails to initialize
    pub fn with_num_threads(num_threads: usize) -> Result<Self> {
        let pool = ThreadPoolBuilder::new()
            .num_threads(num_threads)
            .build()
            .map_err(|e| anyhow::anyhow!("Failed to create thread pool: {}", e))?;

        Ok(Self { pool, shared: Arc::new(SharedState::new()) })
    }

    /// Processes hints in parallel with non-blocking, ordered output.
    ///
    /// This method dispatches each hint to the thread pool for parallel processing.
    /// Results are collected in a reorder buffer and drained (printed) in the original
    /// order as soon as consecutive results become available.
    ///
    /// # Key characteristics:
    /// - **Non-blocking**: Returns immediately after dispatching work to the pool
    /// - **Global sequence**: Sequence IDs are maintained across multiple calls
    /// - **Ordered output**: Results are printed in the order hints were received
    /// - **Error handling**: Stops processing on first error
    ///
    /// # Arguments
    ///
    /// * `hints` - A slice of `u64` values containing concatenated hints
    ///
    /// # Returns
    ///
    /// * `Ok(())` - Hints were successfully dispatched (does not mean processing is complete)
    /// * `Err` - If a previous error occurred or hints are malformed
    pub fn process_hints(&self, hints: &[u64]) -> Result<()> {
        // Check if a previous error occurred
        if self.shared.has_error.load(Ordering::Acquire) {
            return Err(anyhow::anyhow!("Processing stopped due to previous error"));
        }

        // Parse hints and dispatch to pool
        let mut idx = 0;
        while idx < hints.len() {
            // Check for error before processing each hint
            if self.shared.has_error.load(Ordering::Acquire) {
                return Err(anyhow::anyhow!("Processing stopped due to previous error"));
            }

            let hint = PrecompileHint::from_u64_slice(hints, idx)?;
            let length = hint.data.len();

            // Decode stream control from high byte
            let ctrl = (hint.hint_type & STREAM_CTRL_MASK) >> STREAM_CTRL_SHIFT;
            let base_type = hint.hint_type & STREAM_BASE_MASK;

            // Apply stream control actions
            match ctrl {
                STREAM_CTRL_START => {
                    // Reset global sequence and buffer at stream start
                    self.reset();
                    // Control hint only; skip processing
                    idx += length + 1;
                    continue;
                }
                STREAM_CTRL_CANCEL => {
                    // Cancel current stream: set error and notify
                    self.shared.has_error.store(true, Ordering::Release);
                    self.shared.buffer_empty.notify_all();
                    return Err(anyhow::anyhow!("Stream cancelled"));
                }
                STREAM_CTRL_ERROR => {
                    // External error signal
                    self.shared.has_error.store(true, Ordering::Release);
                    self.shared.buffer_empty.notify_all();
                    return Err(anyhow::anyhow!("Stream error signalled"));
                }
                STREAM_CTRL_END => {
                    // Control hint only; wait for completion then skip processing
                    self.wait_for_completion()?;
                    idx += length + 1;
                    continue;
                }
                _ => {}
            }

            // Atomically reserve slot and capture generation inside mutex
            // This prevents orphaned slots if reset happens between generation load and push_back
            let (generation, seq_id) = {
                let mut reorder = self.shared.reorder.lock().unwrap();
                let gen = self.shared.generation.load(Ordering::SeqCst);
                let seq = self.shared.next_seq.fetch_add(1, Ordering::SeqCst);
                reorder.buffer.push_back(None);
                (gen, seq)
            };

            // Spawn processing task
            let shared = Arc::clone(&self.shared);
            self.pool.spawn(move || {
                // Check if we should stop due to error
                if shared.has_error.load(Ordering::Acquire) {
                    return;
                }

                // Process the hint
                // Override hint type to base type for processing
                let mut hint_for_proc = hint;
                hint_for_proc.hint_type = base_type;
                let result = Self::process_hint(&hint_for_proc);

                // Store result and try to drain
                let mut reorder = shared.reorder.lock().unwrap();

                // Check generation first to detect stale workers from previous sessions
                let current_gen = shared.generation.load(Ordering::SeqCst);
                if generation != current_gen {
                    // Worker belongs to old generation; ignore result
                    return;
                }

                // Calculate offset in buffer; handle resets and drained slots
                if seq_id < reorder.base_seq {
                    // This result belongs to a previous stream/session; ignore
                    return;
                }
                let offset = seq_id - reorder.base_seq;
                if offset >= reorder.buffer.len() {
                    // Buffer no longer has a slot for this seq (likely after reset); ignore
                    return;
                }

                // Check error flag again before storing to avoid processing after error
                if shared.has_error.load(Ordering::Acquire) {
                    return;
                }

                reorder.buffer[offset] = Some(result);

                // Drain consecutive ready results from the front
                while let Some(Some(res)) = reorder.buffer.front() {
                    match res {
                        Ok(_data) => {
                            // Print the result (will be replaced with send to another process)
                            // println!("[seq={}] Result: {:?}", reorder.base_seq, data);
                            reorder.buffer.pop_front();
                            reorder.base_seq += 1;
                        }
                        Err(_) => {
                            // Error found - signal to stop and break
                            shared.has_error.store(true, Ordering::Release);
                            // Print error and stop draining
                            if let Some(Some(Err(e))) = reorder.buffer.pop_front() {
                                eprintln!("[seq={}] Error: {}", reorder.base_seq, e);
                            }
                            reorder.base_seq += 1;
                            shared.buffer_empty.notify_all();
                            break;
                        }
                    }
                }

                // Notify if buffer is now empty
                if reorder.buffer.is_empty() {
                    shared.buffer_empty.notify_all();
                }
            });

            idx += length + 1;
        }

        Ok(())
    }

    /// Waits for all pending hints to be processed and drained.
    ///
    /// This method blocks until the reorder buffer is empty, meaning all
    /// dispatched hints have been processed and their results printed.
    ///
    /// # Returns
    ///
    /// * `Ok(())` - All hints processed successfully
    /// * `Err` - If an error occurred during processing
    fn wait_for_completion(&self) -> Result<()> {
        let mut reorder = self.shared.reorder.lock().unwrap();

        while !reorder.buffer.is_empty() {
            if self.shared.has_error.load(Ordering::Acquire) {
                return Err(anyhow::anyhow!("Processing stopped due to error"));
            }
            // Wait for notification that buffer state changed
            reorder = self.shared.buffer_empty.wait(reorder).unwrap();
        }

        if self.shared.has_error.load(Ordering::Acquire) {
            return Err(anyhow::anyhow!("Processing stopped due to error"));
        }

        Ok(())
    }

    /// Resets the processor state, clearing any errors and the reorder buffer.
    ///
    /// This should be called to start a fresh processing session after an error
    /// or when you want to reset the global sequence counter.
    ///
    /// Increments the generation counter to invalidate any in-flight workers
    /// from the previous session, preventing them from corrupting the new state.
    fn reset(&self) {
        self.shared.has_error.store(false, Ordering::Release);
        self.shared.next_seq.store(0, Ordering::Release);
        // Increment generation to invalidate stale workers
        self.shared.generation.fetch_add(1, Ordering::SeqCst);
        let mut reorder = self.shared.reorder.lock().unwrap();
        reorder.buffer.clear();
        reorder.base_seq = 0;
    }

    /// Dispatches a single hint to its appropriate handler based on hint type.
    ///
    /// # Arguments
    ///
    /// * `hint` - The parsed hint to process
    ///
    /// # Returns
    ///
    /// * `Ok(Vec<u64>)` - The processed result for this hint
    /// * `Err` - If the hint type is unknown
    fn process_hint(hint: &PrecompileHint) -> Result<Vec<u64>> {
        let result = match hint.hint_type {
            HINTS_TYPE_RESULT => Self::process_hint_result(hint)?,
            HINTS_TYPE_ECRECOVER => Self::process_hint_ecrecover(hint)?,
            _ => {
                return Err(anyhow::anyhow!("Unknown hint type: {}", hint.hint_type));
            }
        };

        Ok(result)
    }

    /// Processes a [`HINTS_TYPE_RESULT`] hint.
    ///
    /// This is a pass-through handler that simply returns the hint data as-is.
    /// Used when the hint already contains the precomputed result.
    fn process_hint_result(hint: &PrecompileHint) -> Result<Vec<u64>> {
        Ok(hint.data.to_vec())
    }

    /// Processes a [`HINTS_TYPE_ECRECOVER`] hint.
    fn process_hint_ecrecover(_hint: &PrecompileHint) -> Result<Vec<u64>> {
        // TODO!
        // assert!(
        //     hint_length == 8 + 4 + 4 + 4,
        //     "process_hints() Invalid ECRECOVER hint length: {}",
        //     hint_length
        // );
        // let pk: &SyscallPoint256 = unsafe { &(hint.data[i] as const SyscallPoint256) };
        // let z: &[u64; 4] = unsafe { &(hints[i + 8] as const [u64; 4]) };
        // let r: &[u64; 4] = unsafe { &(hints[i + 8 + 4] as const [u64; 4]) };
        // let s: &[u64; 4] = unsafe { &(hints[i + 8 + 4 + 4] as const [u64; 4]) };
        // secp256k1_ecdsa_verify(pk, z, r, s, &mut processedhints);

        Ok(vec![])
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_header(hint_type: u32, length: u32) -> u64 {
        ((hint_type as u64) << 32) | (length as u64)
    }

    fn make_header_with_ctrl(base_type: u32, ctrl: u32, length: u32) -> u64 {
        let hint_type = ((ctrl & 0xFF) << STREAM_CTRL_SHIFT) | (base_type & STREAM_BASE_MASK);
        make_header(hint_type, length)
    }

    fn processor() -> PrecompileHintsProcessor {
        PrecompileHintsProcessor::with_num_threads(2).unwrap()
    }

    // Positive tests
    #[test]
    fn test_single_result_hint_non_blocking() {
        let p = processor();
        let data = vec![make_header(HINTS_TYPE_RESULT, 2), 0x111, 0x222];

        // Dispatch should succeed and be non-blocking
        p.process_hints(&data).unwrap();
        // Wait for completion
        p.wait_for_completion().unwrap();
    }

    #[test]
    fn test_multiple_hints_ordered_output() {
        let p = processor();
        let data = vec![
            make_header(HINTS_TYPE_RESULT, 1),
            0x111,
            make_header(HINTS_TYPE_RESULT, 1),
            0x222,
            make_header(HINTS_TYPE_RESULT, 1),
            0x333,
        ];
        p.process_hints(&data).unwrap();
        p.wait_for_completion().unwrap();
    }

    #[test]
    fn test_multiple_calls_global_sequence() {
        let p = processor();
        let data1 = vec![make_header(HINTS_TYPE_RESULT, 1), 0xAAA];
        let data2 = vec![make_header(HINTS_TYPE_RESULT, 1), 0xBBB];
        p.process_hints(&data1).unwrap();
        p.process_hints(&data2).unwrap();
        p.wait_for_completion().unwrap();
    }

    #[test]
    fn test_empty_input_ok() {
        let p = processor();
        let data: Vec<u64> = vec![];
        p.process_hints(&data).unwrap();
        p.wait_for_completion().unwrap();
    }

    // Negative tests
    #[test]
    fn test_unknown_hint_type_returns_error() {
        let p = processor();
        let data = vec![make_header(999, 1), 0x1234];
        // Dispatch enqueues work; error surfaces on wait
        p.process_hints(&data).unwrap();
        let err = p.wait_for_completion().err().unwrap();
        assert!(err.to_string().contains("error"));
    }

    #[test]
    fn test_error_stops_wait() {
        let p = processor();
        // First valid, then invalid type
        let data = vec![make_header(HINTS_TYPE_RESULT, 1), 0x111, make_header(999, 0)];
        // Dispatch returns error at parse/process of bad hint
        let _ = p.process_hints(&data);
        // Wait should report error state
        let w = p.wait_for_completion();
        assert!(w.is_err());
    }

    #[test]
    fn test_reset_clears_error() {
        let p = processor();
        let bad = vec![make_header(999, 0)];
        let _ = p.process_hints(&bad);
        // Give workers a moment (no busy wait; optional)
        std::thread::sleep(std::time::Duration::from_millis(5));
        p.reset();

        let good = vec![make_header(HINTS_TYPE_RESULT, 1), 0x42];
        p.process_hints(&good).unwrap();
        p.wait_for_completion().unwrap();
    }

    // Stream control tests
    #[test]
    fn test_stream_start_resets_state() {
        let p = processor();
        // First batch increments sequence
        let batch1 = vec![make_header(HINTS_TYPE_RESULT, 1), 0x01];
        p.process_hints(&batch1).unwrap();

        // Send START control; then a new hint
        let start = vec![make_header_with_ctrl(HINTS_TYPE_RESULT, STREAM_CTRL_START, 0)];
        p.process_hints(&start).unwrap();

        let batch2 = vec![make_header(HINTS_TYPE_RESULT, 1), 0x02];
        p.process_hints(&batch2).unwrap();
        // End the stream to ensure completion
        let end = vec![make_header_with_ctrl(HINTS_TYPE_RESULT, STREAM_CTRL_END, 0)];
        p.process_hints(&end).unwrap();
        p.wait_for_completion().unwrap();
    }

    #[test]
    fn test_stream_end_waits_until_completion() {
        let p = processor();
        // Dispatch a few hints
        let data =
            vec![make_header(HINTS_TYPE_RESULT, 1), 0x10, make_header(HINTS_TYPE_RESULT, 1), 0x20];
        p.process_hints(&data).unwrap();
        // End control should cause internal wait during processing
        let end = vec![make_header_with_ctrl(HINTS_TYPE_RESULT, STREAM_CTRL_END, 0)];
        p.process_hints(&end).unwrap();
        // Subsequent explicit wait should be fast (already drained)
        p.wait_for_completion().unwrap();
    }

    #[test]
    fn test_stream_cancel_returns_error() {
        let p = processor();
        let cancel = vec![make_header_with_ctrl(HINTS_TYPE_RESULT, STREAM_CTRL_CANCEL, 0)];
        let err = p.process_hints(&cancel).err().unwrap();
        assert!(err.to_string().contains("cancelled"));
    }

    #[test]
    fn test_stream_error_signal_returns_error() {
        let p = processor();
        let signal_err = vec![make_header_with_ctrl(HINTS_TYPE_RESULT, STREAM_CTRL_ERROR, 0)];
        let err = p.process_hints(&signal_err).err().unwrap();
        assert!(err.to_string().contains("error"));
    }

    // Stress test
    #[test]
    fn test_stress_throughput() {
        use std::time::Instant;

        let p = PrecompileHintsProcessor::with_num_threads(32).unwrap();

        // Generate a large batch of hints
        const NUM_HINTS: usize = 100_000;
        let mut data = Vec::with_capacity(NUM_HINTS * 2);

        for i in 0..NUM_HINTS {
            data.push(make_header(HINTS_TYPE_RESULT, 1));
            data.push(i as u64);
        }

        let start = Instant::now();
        p.process_hints(&data).unwrap();
        p.wait_for_completion().unwrap();
        let duration = start.elapsed();

        let ops_per_sec = NUM_HINTS as f64 / duration.as_secs_f64();
        println!("\n========================================");
        println!("Stress Test Results:");
        println!("  Total hints: {}", NUM_HINTS);
        println!("  Duration: {:.3}s", duration.as_secs_f64());
        println!("  Throughput: {:.0} ops/sec", ops_per_sec);
        println!("  Avg latency: {:.2}µs per hint", duration.as_micros() as f64 / NUM_HINTS as f64);
        println!("========================================\n");

        // Sanity check: should be able to process at least 10k ops/sec
        assert!(ops_per_sec > 10_000.0, "Throughput too low: {:.0} ops/sec", ops_per_sec);
    }

    #[test]
    fn test_stress_concurrent_batches() {
        use std::time::Instant;

        let p = PrecompileHintsProcessor::with_num_threads(32).unwrap();

        const NUM_BATCHES: usize = 1_000;
        const HINTS_PER_BATCH: usize = 100;

        let start = Instant::now();

        // Call process_hints multiple times with small batches
        for batch_id in 0..NUM_BATCHES {
            let mut data = Vec::with_capacity(HINTS_PER_BATCH * 2);
            for i in 0..HINTS_PER_BATCH {
                data.push(make_header(HINTS_TYPE_RESULT, 1));
                data.push((batch_id * HINTS_PER_BATCH + i) as u64);
            }
            p.process_hints(&data).unwrap();
        }

        p.wait_for_completion().unwrap();
        let duration = start.elapsed();

        let total_hints = NUM_BATCHES * HINTS_PER_BATCH;
        let ops_per_sec = total_hints as f64 / duration.as_secs_f64();

        println!("\n========================================");
        println!("Multiple Batches Stress Test:");
        println!("  Number of batches: {}", NUM_BATCHES);
        println!("  Hints per batch: {}", HINTS_PER_BATCH);
        println!("  Total hints: {}", total_hints);
        println!("  Duration: {:.3}s", duration.as_secs_f64());
        println!("  Throughput: {:.0} ops/sec", ops_per_sec);
        println!("========================================\n");

        assert!(ops_per_sec > 10_000.0, "Throughput too low: {:.0} ops/sec", ops_per_sec);
    }

    #[test]
    fn test_stress_with_resets() {
        use std::time::Instant;

        let p = PrecompileHintsProcessor::with_num_threads(32).unwrap();

        const ITERATIONS: usize = 100;
        const HINTS_PER_ITER: usize = 1_000;

        let start = Instant::now();

        for _iter in 0..ITERATIONS {
            // Reset at start of each iteration
            let reset = vec![make_header_with_ctrl(HINTS_TYPE_RESULT, STREAM_CTRL_START, 0)];
            p.process_hints(&reset).unwrap();

            // Process batch
            let mut data = Vec::with_capacity(HINTS_PER_ITER * 2);
            for i in 0..HINTS_PER_ITER {
                data.push(make_header(HINTS_TYPE_RESULT, 1));
                data.push(i as u64);
            }
            p.process_hints(&data).unwrap();

            // End stream
            let end = vec![make_header_with_ctrl(HINTS_TYPE_RESULT, STREAM_CTRL_END, 0)];
            p.process_hints(&end).unwrap();
        }

        let duration = start.elapsed();
        let total_hints = ITERATIONS * HINTS_PER_ITER;
        let ops_per_sec = total_hints as f64 / duration.as_secs_f64();

        println!("\n========================================");
        println!("Reset Stress Test:");
        println!("  Iterations: {}", ITERATIONS);
        println!("  Hints per iteration: {}", HINTS_PER_ITER);
        println!("  Total hints: {}", total_hints);
        println!("  Duration: {:.3}s", duration.as_secs_f64());
        println!("  Throughput: {:.0} ops/sec", ops_per_sec);
        println!("========================================\n");

        assert!(
            ops_per_sec > 5_000.0,
            "Throughput too low with resets: {:.0} ops/sec",
            ops_per_sec
        );
    }
}
