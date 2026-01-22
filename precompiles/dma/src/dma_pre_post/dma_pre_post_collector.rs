//! The `DmaPrePostCollector` module defines an collector to calculate all inputs of an instance
//! for the DmaPrePost State Machine.

use std::{any::Any, collections::VecDeque};

use zisk_common::{BusDevice, BusId, CollectCounter, MemCollectorInfo, OPERATION_BUS_ID, OP_TYPE};
use zisk_core::ZiskOperationType;

use crate::DmaPrePostInput;

pub struct DmaPrePostCollector {
    /// Collected inputs for witness computation.
    pub inputs: Vec<DmaPrePostInput>,

    /// The number of operations to collect.
    pub num_operations: u64,

    /// Helper to skip instructions based on the plan's configuration.
    pub collect_counter: CollectCounter,
}

impl DmaPrePostCollector {
    /// Creates a new `DmaPrePostCollector`.
    ///
    /// # Arguments
    ///
    /// * `bus_id` - The connected bus ID.
    /// * `num_operations` - The number of operations to collect.
    /// * `collect_skipper` - The helper to skip instructions based on the plan's configuration.
    ///
    /// # Returns
    /// A new `ArithInstanceCollector` instance initialized with the provided parameters.
    pub fn new(num_operations: u64, collect_counter: CollectCounter) -> Self {
        Self {
            inputs: Vec::with_capacity(num_operations as usize),
            num_operations,
            collect_counter,
        }
    }
}

impl BusDevice<u64> for DmaPrePostCollector {
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

        if self.inputs.len() == self.num_operations as usize {
            return false;
        }

        if data[OP_TYPE] != ZiskOperationType::Dma as u64 {
            return true;
        }

        // println!(
        //     "DmaPrePostCollector::process_data {} {:?}",
        //     DmaInfo::to_string(data[DMA_ENCODED]),
        //     self.collect_counter
        // );
        let rows = DmaPrePostInput::get_count(data);
        let res = self.collect_counter.should_process(rows as u32);
        // println!("DmaPrePostCollector::process_data2 {} {:?}", rows, res);
        if let Some((skip, max_count)) = res {
            self.inputs.extend(DmaPrePostInput::from(data, data_ext, skip, max_count));
        }
        // println!("DmaPrePostCollector::process_data3 input.len()={}", self.inputs.len());
        self.inputs.len() < self.num_operations as usize
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
