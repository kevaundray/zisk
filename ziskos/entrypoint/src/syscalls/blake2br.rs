//! Blake2br system call interception

#[cfg(feature = "zisk_guest")]
use core::arch::asm;

#[cfg(feature = "zisk_guest")]
use crate::ziskos_syscall;

#[cfg(not(feature = "zisk_guest"))]
use precompiles_helpers::blake2b_round;

#[derive(Debug)]
#[repr(C)]
pub struct SyscallBlake2bRoundParams<'a> {
    pub index: u64, // a number in [0,10)
    pub state: &'a mut [u64; 16],
    pub input: &'a [u64; 16],
}

#[allow(unused_variables)]
#[cfg_attr(not(feature = "hints"), no_mangle)]
#[cfg_attr(feature = "hints", export_name = "hints_syscall_blake2b_round")]
pub extern "C" fn syscall_blake2b_round(
    params: &mut SyscallBlake2bRoundParams,
    #[cfg(feature = "hints")] hints: &mut Vec<u64>,
) {
    #[cfg(feature = "zisk_guest")]
    ziskos_syscall!(zisk_definitions::SYSCALL_BLAKE2B_ROUND_ID, params);

    #[cfg(not(feature = "zisk_guest"))]
    {
        blake2b_round(params.state, params.input, params.index as u32);

        #[cfg(feature = "hints")]
        {
            hints.extend_from_slice(params.state);
        }
    }
}
