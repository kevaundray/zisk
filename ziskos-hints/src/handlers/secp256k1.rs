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
pub fn secp256k1_ecdsa_verify_hint(data: &[u64]) -> Result<Vec<u64>, String> {
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

// Processes a SECP256K1_TO_AFFINE hint.
#[inline]
pub fn secp256k1_to_affine_hint(data: &[u64]) -> Result<Vec<u64>, String> {
    hint_fields![P: 12];

    validate_hint_length(data, EXPECTED_LEN, "SECP256K1_TO_AFFINE")?;

    let mut out: [u64; 8] = [0; 8];
    let mut processed_hints = Vec::new();

    unsafe {
        zisklib::secp256k1_to_affine_c(&data[P_OFFSET], &mut out[0], &mut processed_hints);
    }

    Ok(processed_hints)
}

// Processes a SECP256K1_DECOMPRESS hint.
#[inline]
pub fn secp256k1_decompress_hint(data: &[u64]) -> Result<Vec<u64>, String> {
    hint_fields![X_BYTES: 4, Y_IS_ODD: 1];

    validate_hint_length(data, EXPECTED_LEN, "SECP256K1_DECOMPRESS")?;

    let mut out: [u64; 8] = [0; 8];
    let mut processed_hints = Vec::new();

    unsafe {
        zisklib::secp256k1_decompress_c(
            &data[X_BYTES_OFFSET] as *const u64 as *const u8,
            (data[Y_IS_ODD_OFFSET] >> 56) as u8,
            &mut out[0],
            &mut processed_hints,
        );
    }

    Ok(processed_hints)
}

// Processes a SECP256K1_DOUBLE_SCALAR_MUL_WITH_G hint.
#[inline]
pub fn secp256k1_double_scalar_mul_with_g_hint(data: &[u64]) -> Result<Vec<u64>, String> {
    hint_fields![K1: 4, K2: 4, P: 8];

    validate_hint_length(data, EXPECTED_LEN, "SECP256K1_DOUBLE_SCALAR_MUL_WITH_G")?;

    let mut out: [u64; 8] = [0; 8];
    let mut processed_hints = Vec::new();

    unsafe {
        zisklib::secp256k1_double_scalar_mul_with_g_c(
            &data[K1_OFFSET],
            &data[K2_OFFSET],
            &data[P_OFFSET],
            &mut out[0],
            &mut processed_hints,
        );
    }

    Ok(processed_hints)
}

// Processes a SECP256K1_FP_REDUCE hint.
#[inline]
pub fn secp256k1_fp_reduce_hint(data: &[u64]) -> Result<Vec<u64>, String> {
    hint_fields![X: 4];

    validate_hint_length(data, EXPECTED_LEN, "SECP256K1_FP_REDUCE")?;

    let mut out: [u64; 4] = [0; 4];
    let mut processed_hints = Vec::new();

    unsafe {
        zisklib::secp256k1_fp_reduce_c(&data[X_OFFSET], &mut out[0], &mut processed_hints);
    }

    Ok(processed_hints)
}

// Processes a SECP256K1_FP_ADD hint.
#[inline]
pub fn secp256k1_fp_add_hint(data: &[u64]) -> Result<Vec<u64>, String> {
    hint_fields![X: 4, Y: 4];

    validate_hint_length(data, EXPECTED_LEN, "SECP256K1_FP_ADD")?;

    let mut out: [u64; 4] = [0; 4];
    let mut processed_hints = Vec::new();

    unsafe {
        zisklib::secp256k1_fp_add_c(
            &data[X_OFFSET],
            &data[Y_OFFSET],
            &mut out[0],
            &mut processed_hints,
        );
    }

    Ok(processed_hints)
}

// Processes a SECP256K1_FP_NEGATE hint.
#[inline]
pub fn secp256k1_fp_negate_hint(data: &[u64]) -> Result<Vec<u64>, String> {
    hint_fields![X: 4];

    validate_hint_length(data, EXPECTED_LEN, "SECP256K1_FP_NEGATE")?;

    let mut out: [u64; 4] = [0; 4];
    let mut processed_hints = Vec::new();

    unsafe {
        zisklib::secp256k1_fp_negate_c(&data[X_OFFSET], &mut out[0], &mut processed_hints);
    }

    Ok(processed_hints)
}

// Processes a SECP256K1_FP_MUL hint.
#[inline]
pub fn secp256k1_fp_mul_hint(data: &[u64]) -> Result<Vec<u64>, String> {
    hint_fields![X: 4, Y: 4];

    validate_hint_length(data, EXPECTED_LEN, "SECP256K1_FP_MUL")?;

    let mut out: [u64; 4] = [0; 4];
    let mut processed_hints = Vec::new();

    unsafe {
        zisklib::secp256k1_fp_mul_c(
            &data[X_OFFSET],
            &data[Y_OFFSET],
            &mut out[0],
            &mut processed_hints,
        );
    }

    Ok(processed_hints)
}

// Processes a SECP256K1_FP_MUL_SCALAR hint.
#[inline]
pub fn secp256k1_fp_mul_scalar_hint(data: &[u64]) -> Result<Vec<u64>, String> {
    hint_fields![X: 4, SCALAR: 1];

    validate_hint_length(data, EXPECTED_LEN, "SECP256K1_FP_MUL_SCALAR")?;

    let mut out: [u64; 4] = [0; 4];
    let mut processed_hints = Vec::new();

    unsafe {
        zisklib::secp256k1_fp_mul_scalar_c(
            &data[X_OFFSET],
            data[SCALAR_OFFSET],
            &mut out[0],
            &mut processed_hints,
        );
    }

    Ok(processed_hints)
}
