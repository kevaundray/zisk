use std::sync::Arc;

use fields::PrimeField64;
use pil_std_lib::Std;

use proofman_common::{AirInstance, FromTrace, ProofmanResult};
use proofman_util::{timer_start_trace, timer_stop_and_log_trace};

#[cfg(not(feature = "packed"))]
use zisk_pil::{KeccakfTrace, KeccakfTraceRow};
#[cfg(feature = "packed")]
use zisk_pil::{KeccakfTracePacked, KeccakfTraceRowPacked};

#[cfg(feature = "packed")]
type KeccakfTraceType<F> = KeccakfTracePacked<F>;
#[cfg(feature = "packed")]
type KeccakfTraceRowType<F> = KeccakfTraceRowPacked<F>;

#[cfg(not(feature = "packed"))]
type KeccakfTraceType<F> = KeccakfTrace<F>;
#[cfg(not(feature = "packed"))]
type KeccakfTraceRowType<F> = KeccakfTraceRow<F>;

use precompiles_helpers::{keccak_f_rounds, keccakf_state_from_linear, keccakf_state_to_linear_1d};

use crate::KeccakfInput;

use super::{keccakf_constants::*, KeccakfTableSM};

use rayon::prelude::*;

/// The `KeccakfSM` struct encapsulates the logic of the Keccakf State Machine.
pub struct KeccakfSM<F: PrimeField64> {
    /// Number of available keccakfs in the trace.
    pub num_available_keccakfs: usize,

    /// Reference to the PIL2 standard library.
    std: Arc<Std<F>>,

    /// The table ID for the Keccakf Table State Machine
    table_id: usize,
}

impl<F: PrimeField64> KeccakfSM<F> {
    /// Creates a new Keccakf State Machine instance.
    ///
    /// # Arguments
    /// * `keccakf_table_sm` - An `Arc`-wrapped reference to the Keccakf Table State Machine.
    ///
    /// # Returns
    /// A new `KeccakfSM` instance.
    pub fn new(std: Arc<Std<F>>) -> Arc<Self> {
        // Compute some useful values
        let num_non_usable_rows = KeccakfTraceType::<F>::NUM_ROWS % CLOCKS;
        let num_available_keccakfs = if num_non_usable_rows == 0 {
            KeccakfTraceType::<F>::NUM_ROWS / CLOCKS
        } else {
            // Subtract 1 because we can't fit a complete cycle in the remaining rows
            (KeccakfTraceType::<F>::NUM_ROWS - num_non_usable_rows) / CLOCKS - 1
        };

        // Get the table ID
        let table_id = std
            .get_virtual_table_id(KeccakfTableSM::TABLE_ID)
            .expect("Failed to get Keccakf table ID");

        Arc::new(Self { num_available_keccakfs, std, table_id })
    }

    /// Processes a slice of operation data, updating the trace and multiplicities.
    ///
    /// # Arguments
    /// * `trace` - A mutable reference to the Keccakf trace.
    /// * `input` - The operation data to process.
    #[inline(always)]
    #[allow(clippy::needless_range_loop)]
    fn process_trace(
        &self,
        trace: &mut [KeccakfTraceRowType<F>],
        initial_state: &[u64; 25],
        addr: u32,
        step: u64,
    ) -> Vec<[u32; NUM_CHUNKS]> {
        // Fill step and addr
        trace[0].set_step_addr(step);
        trace[1].set_step_addr(addr as u64);

        // Fill in_use
        for i in 0..CLOCKS {
            trace[i].set_in_use(true);
        }

        // Collect accumulators to avoid recomputation
        let mut chunk_accs = Vec::with_capacity(ROUNDS);

        // Convert input state to 5x5x64 representation
        let initial_state = keccakf_state_from_linear(initial_state);
        let round_states = keccak_f_rounds(initial_state);

        // Fill the states
        for (state_3d, r) in round_states {
            let state_1d = keccakf_state_to_linear_1d(&state_3d);

            // Fill state
            for i in 0..WIDTH {
                trace[r].set_state(i, (state_1d[i] % 2) == 1);
            }

            // Compute accumulators for lookup
            if r > 0 {
                let mut accs = [0u32; NUM_CHUNKS];
                for i in 0..NUM_CHUNKS {
                    let offset = i * TABLE_MAX_CHUNKS;
                    let num_bits = std::cmp::min(TABLE_MAX_CHUNKS, WIDTH - offset);

                    let mut acc = 0u32;
                    for j in 0..num_bits {
                        acc += (state_1d[offset + j] as u32) * BASE.pow(j as u32);
                    }
                    accs[i] = acc;

                    // Set the accumulator
                    trace[r - 1].set_chunk_acc(i, acc);
                }
                chunk_accs.push(accs);
            }
        }

        chunk_accs
    }

    /// Computes the witness for a series of inputs and produces an `AirInstance`.
    ///
    /// # Arguments
    /// * `inputs` - A slice of operations to process.
    ///
    /// # Returns
    /// An `AirInstance` containing the computed witness data.
    pub fn compute_witness(
        &self,
        inputs: &[Vec<KeccakfInput>],
        trace_buffer: Vec<F>,
    ) -> ProofmanResult<AirInstance<F>> {
        let mut trace = KeccakfTraceType::new_from_vec_zeroes(trace_buffer)?;
        let num_rows = trace.num_rows();

        // Check that we can fit all the keccakfs in the trace
        let num_available_keccakfs = self.num_available_keccakfs;
        let num_inputs = inputs.iter().map(|v| v.len()).sum::<usize>();
        let num_rows_needed = if num_inputs < num_available_keccakfs {
            num_inputs * CLOCKS
        } else if num_inputs == num_available_keccakfs {
            num_rows
        } else {
            panic!(
                "Exceeded available Keccakfs inputs: requested {}, but only {} are available.",
                num_inputs, num_available_keccakfs
            );
        };

        tracing::debug!(
            "··· Creating Keccakf instance [{} / {} rows filled {:.2}%]",
            num_rows_needed,
            num_rows,
            num_rows_needed as f64 / num_rows as f64 * 100.0
        );

        timer_start_trace!(KECCAKF_TRACE);

        // 1] Fill the trace with the provided inputs
        let mut trace_rows = &mut trace.buffer[..];
        let mut par_traces = Vec::new();
        let mut inputs_indexes = Vec::new();
        for (i, inputs) in inputs.iter().enumerate() {
            for (j, _) in inputs.iter().enumerate() {
                let (head, tail) = trace_rows.split_at_mut(CLOCKS);
                par_traces.push(head);
                inputs_indexes.push((i, j));
                trace_rows = tail;
            }
        }

        let chunk_accs: Vec<_> = par_traces
            .par_iter_mut()
            .enumerate()
            .map(|(index, trace)| {
                let input_index = inputs_indexes[index];
                let input = &inputs[input_index.0][input_index.1];
                self.process_trace(trace, &input.state, input.addr_main, input.step_main)
            })
            .collect();

        // 2] Update lookup table
        let mut table = vec![0u32; TABLE_SIZE as usize];
        for accs_per_keccakf in &chunk_accs {
            for round_accs in accs_per_keccakf {
                for &acc in round_accs {
                    let table_row = KeccakfTableSM::calculate_table_row(acc);
                    table[table_row as usize] += 1;
                }
            }
        }
        table.into_par_iter().enumerate().for_each(|(row, value)| {
            if value > 0 {
                self.std.inc_virtual_row(self.table_id, row as u64, value as u64);
            }
        });
        timer_stop_and_log_trace!(KECCAKF_TRACE);

        Ok(AirInstance::new_from_trace(FromTrace::new(&mut trace)))
    }
}
