use anyhow::{anyhow, Ok, Result};
use proofman_verifier::verify;

pub fn verify_zisk_proof(zisk_proof: &[u8], vk: &[u8]) -> Result<()> {
    if !verify(zisk_proof, vk) {
        Err(anyhow!("Zisk Proof was not verified"))
    } else {
        Ok(())
    }
}
