//! The `Dma64AlignedInstance` module defines an instance to perform the witness computation
//! for the Dma State Machine.
//!
//! It manages collected inputs and interacts with the `DmaSM` to compute witnesses for
//! execution plans.

use crate::Dma64AlignedInput;
use std::any::Any;
use std::collections::VecDeque;
use zisk_common::{BusDevice, BusId, CollectCounter, MemCollectorInfo, OPERATION_BUS_ID, OP_TYPE};
use zisk_core::ZiskOperationType;

pub struct Dma64AlignedCollector {
    /// Collected inputs for witness computation.
    pub inputs: Vec<Dma64AlignedInput>,

    /// The number of inputs to collect.
    pub num_inputs: u64,

    /// Helper to skip instructions based on the plan's configuration.
    pub collect_counter: CollectCounter,

    pub trace_offset: usize,
    pub last_segment_collector: bool,
}

impl Dma64AlignedCollector {
    /// Creates a new `Dma64AlignedCollector`.
    ///
    /// # Arguments
    ///
    /// * `bus_id` - The connected bus ID.
    /// * `num_inputs` - The number of inputs to collect.
    /// * `collect_counter` - The helper to skip instructions based on the plan's configuration.
    ///
    /// # Returns
    /// A new `Dma64AlignedCollector` instance initialized with the provided parameters.
    pub fn new(
        num_inputs: u64,
        collect_counter: CollectCounter,
        last_segment_collector: bool,
    ) -> Self {
        Self {
            inputs: Vec::with_capacity(num_inputs as usize),
            num_inputs,
            collect_counter,
            trace_offset: 0,
            last_segment_collector,
        }
    }
}

impl BusDevice<u64> for Dma64AlignedCollector {
    /// Processes data received on the bus, collecting the inputs necessary for witness computation.
    ///
    /// # Arguments
    /// * `_bus_id` - The ID of the bus (unused in this implementation).
    /// * `data` - The data received from the bus.
    /// * `pending` – A queue of pending bus operations used to send derived inputs.
    ///
    /// # Returns
    /// A tuple where:
    /// A boolean indicating whether the program should continue execution or terminate.
    /// Returns `true` to continue execution, `false` to stop.
    #[inline(always)]
    fn process_data(
        &mut self,
        bus_id: &BusId,
        data: &[u64],
        data_ext: &[u64],
        _pending: &mut VecDeque<(BusId, Vec<u64>, Vec<u64>)>,
        _mem_collector_info: Option<&[MemCollectorInfo]>,
    ) -> bool {
        debug_assert!(*bus_id == OPERATION_BUS_ID);

        if self.inputs.len() == self.num_inputs as usize {
            return false;
        }

        if data[OP_TYPE] != ZiskOperationType::Dma as u64 {
            return true;
        }

        let rows = Dma64AlignedInput::get_rows(data) as u32;
        if rows == 0 {
            return true;
        }

        if let Some((skip, max_count)) = self.collect_counter.should_process(rows) {
            self.inputs.push(Dma64AlignedInput::from(
                data,
                data_ext,
                self.trace_offset,
                skip as usize,
                max_count as usize,
                self.last_segment_collector && self.collect_counter.is_final_skip(),
            ));
            self.trace_offset += max_count as usize;
        }

        self.inputs.len() < self.num_inputs as usize
    }

    /// Returns the bus IDs associated with this instance.
    ///
    /// # Returns
    /// A vector containing the connected bus ID.
    fn bus_id(&self) -> Vec<BusId> {
        vec![OPERATION_BUS_ID]
    }

    fn as_any(self: Box<Self>) -> Box<dyn Any> {
        self
    }
}
