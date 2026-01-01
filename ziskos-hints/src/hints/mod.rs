//! Hint processing utilities for ziskos-hints

use crate::zisklib;

/// Processes an ECRECOVER hint.
///
/// # Arguments
///
/// * `data` - The hint data containing pk(8) + z(4) + r(4) + s(4) = 20 u64 values
///
/// # Returns
///
/// * `Ok(Vec<u64>)` - The processed hints from the verification
/// * `Err` - If the data length is invalid
#[inline]
pub fn process_ecrecover_hint(data: &[u64]) -> Result<Vec<u64>, String> {
    const PK_SIZE: usize = 8; // x(4) + y(4)
    const Z_SIZE: usize = 4;
    const R_SIZE: usize = 4;
    const S_SIZE: usize = 4;
    const EXPECTED_LEN: usize = PK_SIZE + Z_SIZE + R_SIZE + S_SIZE;

    const Z_OFFSET: usize = PK_SIZE;
    const R_OFFSET: usize = Z_OFFSET + Z_SIZE;
    const S_OFFSET: usize = R_OFFSET + R_SIZE;

    if data.len() != EXPECTED_LEN {
        return Err(format!(
            "Invalid ECRECOVER hint length: expected {}, got {}",
            EXPECTED_LEN,
            data.len()
        ));
    }

    #[allow(unused_mut)]
    let mut processed_hints = Vec::new();

    // Safety: We've validated that data.len() == 20, so all slice accesses are in bounds.
    unsafe {
        let ptr = data.as_ptr();
        let pk = &*ptr;
        let z = &*ptr.add(Z_OFFSET);
        let r = &*ptr.add(R_OFFSET);
        let s = &*ptr.add(S_OFFSET);

        zisklib::secp256k1_ecdsa_verify_c(pk, z, r, s, &mut processed_hints);
    }

    Ok(processed_hints)
}

pub fn process_redmod256_hint(_data: &[u64]) -> Result<Vec<u64>, String> {
    unimplemented!("REDMOD256 hint processing is not yet implemented");
}

pub fn process_addmod256_hint(_data: &[u64]) -> Result<Vec<u64>, String> {
    unimplemented!("ADDMOD256 hint processing is not yet implemented");
}

pub fn process_mulmod256_hint(_data: &[u64]) -> Result<Vec<u64>, String> {
    unimplemented!("MULMOD256 hint processing is not yet implemented");
}

pub fn process_divrem256_hint(_data: &[u64]) -> Result<Vec<u64>, String> {
    unimplemented!("DIVREM256 hint processing is not yet implemented");
}
pub fn process_wpow256_hint(_data: &[u64]) -> Result<Vec<u64>, String> {
    unimplemented!("WPOW256 hint processing is not yet implemented");
}
pub fn process_omul256_hint(_data: &[u64]) -> Result<Vec<u64>, String> {
    unimplemented!("OMUL256 hint processing is not yet implemented");
}
pub fn process_wmul256_hint(_data: &[u64]) -> Result<Vec<u64>, String> {
    unimplemented!("WMUL256 hint processing is not yet implemented");
}
