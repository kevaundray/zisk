use crate::handlers::validate_hint_length;
use crate::hint_fields;
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
    hint_fields![PK: 8, Z: 4, R: 4, S: 4];

    validate_hint_length(data, EXPECTED_LEN, "ECRECOVER")?;

    let mut processed_hints = Vec::new();

    unsafe {
        zisklib::secp256k1_ecdsa_verify_c(
            &data[PK_OFFSET],
            &data[Z_OFFSET],
            &data[R_OFFSET],
            &data[S_OFFSET],
            &mut processed_hints,
        );
    }

    Ok(processed_hints)
}
