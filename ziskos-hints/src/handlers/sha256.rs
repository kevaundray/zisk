use crate::zisklib;

use anyhow::Result;

/// Processes an `HINT_SHA256` hint.
#[inline]
pub fn sha256_hint(data: &[u64], data_len_bytes: usize) -> Result<()> {
    let data_len_words = data_len_bytes.div_ceil(8);

    if data.len() != data_len_words {
        anyhow::bail!(
            "HINT_SHA256: expected data length of {} bytes ({} words), got {} words",
            data_len_bytes,
            data_len_words,
            data.len()
        );
    }

    let bytes = unsafe { std::slice::from_raw_parts(data.as_ptr() as *const u8, data_len_bytes) };

    zisklib::sha256(bytes);

    Ok(())
}
