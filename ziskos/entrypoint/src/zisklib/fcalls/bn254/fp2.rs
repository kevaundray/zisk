use cfg_if::cfg_if;

cfg_if! {
    if #[cfg(feature = "zisk_guest")] {
        use core::arch::asm;
        use crate::{ziskos_fcall, ziskos_fcall_get, ziskos_fcall_param, zisklib::FCALL_BN254_FP2_INV_ID};
    } else {
        use crate::zisklib::fcalls_impl::bn254::bn254_fp2_inv;
    }
}

/// Executes the multiplicative inverse computation over the complex extension field of the `bn254` curve.
///
/// `fcall_bn254_fp2_inv` performs an inversion of a 512-bit extension field element,
/// represented as an array of eight `u64` values.
///
/// - `fcall_bn254_fp2_inv` performs the inversion and **returns the result directly**.
///
/// ### Safety
///
/// The caller must ensure that the input pointer (`p_value`) is valid and aligned to an 8-byte boundary.
///
/// Note that this is a *free-input call*, meaning the Zisk VM does not automatically verify the correctness
/// of the result. It is the caller's responsibility to ensure it.
#[allow(unused_variables)]
pub fn fcall_bn254_fp2_inv(
    p_value: &[u64; 8],
    #[cfg(feature = "hints")] hints: &mut Vec<u64>,
) -> [u64; 8] {
    #[cfg(not(feature = "zisk_guest"))]
    {
        let result: [u64; 8] = bn254_fp2_inv(p_value);
        #[cfg(feature = "hints")]
        {
            hints.push(result.len() as u64);
            hints.extend_from_slice(&result);
        }
        result
    }
    #[cfg(feature = "zisk_guest")]
    {
        ziskos_fcall_param!(p_value, 8);
        ziskos_fcall!(FCALL_BN254_FP2_INV_ID);
        [
            ziskos_fcall_get(),
            ziskos_fcall_get(),
            ziskos_fcall_get(),
            ziskos_fcall_get(),
            ziskos_fcall_get(),
            ziskos_fcall_get(),
            ziskos_fcall_get(),
            ziskos_fcall_get(),
        ]
    }
}
