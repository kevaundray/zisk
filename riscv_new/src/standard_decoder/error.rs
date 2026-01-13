/// Decoder errors
#[derive(Debug, thiserror::Error)]
pub enum DecodeError {
    #[error("Unsupported extension: {0}")]
    UnsupportedExtension(String),

    #[error("Invalid instruction format")]
    InvalidFormat,

    #[error("Instruction not supported by target")]
    UnsupportedInstruction,
}
