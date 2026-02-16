//! Keccak system call interception

#[cfg(feature = "guest")]
use core::arch::asm;

#[cfg(feature = "guest")]
use crate::ziskos_syscall;

#[cfg(not(feature = "guest"))]
use tiny_keccak::keccakf;

/// Executes the Keccak256 permutation on the given state.
///
/// The `Keccak` system call executes a CSR set on a custom port. When transpiling from RISC-V to Zisk,
/// this instruction is replaced with a precompiled operation—specifically, `Keccak`.
///
/// The syscall takes as a parameter the address of a state data (1600 bits = 200 bytes)
/// and the result of the keccakf operation is stored at the same location
///
/// ### Safety
///
/// The caller must ensure that the data is aligned to a 64-bit boundary.
#[allow(unused_variables)]
#[cfg_attr(not(feature = "hints"), no_mangle)]
#[cfg_attr(feature = "hints", export_name = "hints_syscall_keccak_f")]
pub unsafe extern "C" fn syscall_keccak_f(
    state: *mut [u64; 25],
    #[cfg(feature = "hints")] hints: &mut Vec<u64>,
) {
    #[cfg(feature = "guest")]
    ziskos_syscall!(0x800, state);
    #[cfg(not(feature = "guest"))]
    {
        // Call keccakf
        keccakf(unsafe { &mut *state });

        // Store results in hints vector
        #[cfg(feature = "hints")]
        {
            hints.extend_from_slice(unsafe { &*state });
        }
    }
}
