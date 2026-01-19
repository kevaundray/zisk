use crate::{
    handlers::{read_field, validate_hint_length},
    zisklib,
};

use anyhow::Result;

// Processes a `MODEXP` hint.
#[inline]
pub fn modexp_hint(data: &[u64]) -> Result<Vec<u64>> {
    let mut pos = 0;
    let base = read_field(data, &mut pos)?;
    let exp = read_field(data, &mut pos)?;
    let modulus = read_field(data, &mut pos)?;

    validate_hint_length(data, pos, "MODEXP")?;

    let mut hints = Vec::new();
    zisklib::modexp_u64(base, exp, modulus, &mut hints);

    Ok(hints)
}
