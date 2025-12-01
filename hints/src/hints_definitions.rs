// Every hint is preceded by a u64 prefix that contains its type and length
// The upper 32 bits contain the hint type, and the lower 32 bits contain the length

// Hints type constants
pub const HINTS_TYPE_RESULT: u32 = 1; // Data is already the result of the precompile
pub const HINTS_TYPE_ECRECOVER: u32 = 2; // Data is the input for the ecrecover precompile
