use crate::syscalls::{
    syscall_add256, syscall_arith256, syscall_arith256_mod, SyscallAdd256Params,
    SyscallArith256ModParams, SyscallArith256Params,
};
use crate::zisklib::fcall_bigint256_div;

const ZERO: [u64; 4] = [0, 0, 0, 0];
const ONE: [u64; 4] = [1, 0, 0, 0];
const MAX: [u64; 4] = [u64::MAX, u64::MAX, u64::MAX, u64::MAX];
/// 2^255 — minimum signed 256-bit value in two's complement.
const MIN_I256: [u64; 4] = [0, 0, 0, 0x8000_0000_0000_0000];

#[inline]
fn is_zero(a: &[u64; 4]) -> bool {
    a[0] == 0 && a[1] == 0 && a[2] == 0 && a[3] == 0
}

#[inline]
fn is_negative(a: &[u64; 4]) -> bool {
    a[3] & 0x8000_0000_0000_0000 != 0
}

#[inline]
fn lt_u256(a: &[u64; 4], b: &[u64; 4]) -> bool {
    for i in (0..4).rev() {
        if a[i] != b[i] {
            return a[i] < b[i];
        }
    }
    false
}

fn negate(a: &[u64; 4]) -> [u64; 4] {
    let not = [!a[0], !a[1], !a[2], !a[3]];
    let mut result = ZERO;
    let mut params = SyscallAdd256Params { a: &not, b: &ONE, cin: 0, c: &mut result };
    syscall_add256(
        &mut params,
        #[cfg(feature = "hints")]
        &mut Vec::new(),
    );
    result
}

/// Hint-and-verify division. Caller must ensure b != 0.
fn divmod(a: &[u64; 4], b: &[u64; 4]) -> ([u64; 4], [u64; 4]) {
    if lt_u256(a, b) {
        return (ZERO, *a);
    }
    if a == b {
        return (ONE, ZERO);
    }

    let (quo, rem) = fcall_bigint256_div(
        a,
        b,
        #[cfg(feature = "hints")]
        &mut Vec::new(),
    );

    // Verify: q * b + r == a
    let mut dl = ZERO;
    let mut dh = ZERO;
    let mut params = SyscallArith256Params { a: &quo, b, c: &rem, dl: &mut dl, dh: &mut dh };
    syscall_arith256(
        &mut params,
        #[cfg(feature = "hints")]
        &mut Vec::new(),
    );

    debug_assert_eq!(dh, ZERO, "divmod overflow: q*b+r exceeds 256 bits");
    debug_assert_eq!(dl, *a, "divmod verification failed: q*b+r != a");
    debug_assert!(lt_u256(&rem, b), "divmod: remainder >= divisor");

    (quo, rem)
}

/// EVM ADD: (a + b) mod 2^256
#[inline]
pub fn evm_add(a: &[u64; 4], b: &[u64; 4]) -> [u64; 4] {
    let mut result = ZERO;
    let mut params = SyscallAdd256Params { a, b, cin: 0, c: &mut result };
    let _ = syscall_add256(
        &mut params,
        #[cfg(feature = "hints")]
        &mut Vec::new(),
    );
    result
}

/// EVM SUB: (a - b) mod 2^256
#[inline]
pub fn evm_sub(a: &[u64; 4], b: &[u64; 4]) -> [u64; 4] {
    let not_b = [!b[0], !b[1], !b[2], !b[3]];
    let mut result = ZERO;
    let mut params = SyscallAdd256Params { a, b: &not_b, cin: 1, c: &mut result };
    let _ = syscall_add256(
        &mut params,
        #[cfg(feature = "hints")]
        &mut Vec::new(),
    );
    result
}

/// EVM MUL: (a * b) mod 2^256 (low 256 bits)
#[inline]
pub fn evm_mul(a: &[u64; 4], b: &[u64; 4]) -> [u64; 4] {
    let mut dl = ZERO;
    let mut dh = ZERO;
    let mut params = SyscallArith256Params { a, b, c: &ZERO, dl: &mut dl, dh: &mut dh };
    syscall_arith256(
        &mut params,
        #[cfg(feature = "hints")]
        &mut Vec::new(),
    );
    dl
}

/// EVM DIV: a / b (unsigned). Returns 0 if b == 0.
pub fn evm_div(a: &[u64; 4], b: &[u64; 4]) -> [u64; 4] {
    if is_zero(b) {
        return ZERO;
    }
    divmod(a, b).0
}

/// EVM SDIV: signed division. Returns 0 if b == 0.
/// Special case: SDIV(MIN_I256, -1) = MIN_I256
pub fn evm_sdiv(a: &[u64; 4], b: &[u64; 4]) -> [u64; 4] {
    if is_zero(b) {
        return ZERO;
    }
    if *a == MIN_I256 && *b == MAX {
        return MIN_I256;
    }

    let a_neg = is_negative(a);
    let b_neg = is_negative(b);
    let abs_a = if a_neg { negate(a) } else { *a };
    let abs_b = if b_neg { negate(b) } else { *b };

    let (quo, _) = divmod(&abs_a, &abs_b);

    if a_neg != b_neg { negate(&quo) } else { quo }
}

/// EVM MOD: a % b (unsigned). Returns 0 if b == 0.
pub fn evm_mod(a: &[u64; 4], b: &[u64; 4]) -> [u64; 4] {
    if is_zero(b) {
        return ZERO;
    }
    divmod(a, b).1
}

/// EVM SMOD: signed modulo. Returns 0 if b == 0.
/// Result takes the sign of a.
pub fn evm_smod(a: &[u64; 4], b: &[u64; 4]) -> [u64; 4] {
    if is_zero(b) {
        return ZERO;
    }

    let a_neg = is_negative(a);
    let b_neg = is_negative(b);
    let abs_a = if a_neg { negate(a) } else { *a };
    let abs_b = if b_neg { negate(b) } else { *b };

    let (_, rem) = divmod(&abs_a, &abs_b);

    if a_neg { negate(&rem) } else { rem }
}

/// EVM ADDMOD: (a + b) % m. Returns 0 if m == 0.
#[inline]
pub fn evm_addmod(a: &[u64; 4], b: &[u64; 4], m: &[u64; 4]) -> [u64; 4] {
    if is_zero(m) {
        return ZERO;
    }
    let mut result = ZERO;
    let mut params = SyscallArith256ModParams { a, b: &ONE, c: b, module: m, d: &mut result };
    syscall_arith256_mod(
        &mut params,
        #[cfg(feature = "hints")]
        &mut Vec::new(),
    );
    result
}

/// EVM MULMOD: (a * b) % m. Returns 0 if m == 0.
#[inline]
pub fn evm_mulmod(a: &[u64; 4], b: &[u64; 4], m: &[u64; 4]) -> [u64; 4] {
    if is_zero(m) {
        return ZERO;
    }
    let mut result = ZERO;
    let mut params = SyscallArith256ModParams { a, b, c: &ZERO, module: m, d: &mut result };
    syscall_arith256_mod(
        &mut params,
        #[cfg(feature = "hints")]
        &mut Vec::new(),
    );
    result
}

/// EVM EXP: base^exponent mod 2^256
pub fn evm_exp(base: &[u64; 4], exponent: &[u64; 4]) -> [u64; 4] {
    if is_zero(exponent) {
        return ONE;
    }
    if is_zero(base) {
        return ZERO;
    }

    let mut result = ONE;
    let mut b = *base;

    for limb_idx in 0..4 {
        let mut limb = exponent[limb_idx];
        if limb_idx > 0 && limb == 0 && exponent[limb_idx + 1..].iter().all(|&l| l == 0) {
            break;
        }
        for _ in 0..64 {
            if limb & 1 != 0 {
                result = evm_mul(&result, &b);
            }
            limb >>= 1;
            if limb == 0 && exponent[limb_idx + 1..].iter().all(|&l| l == 0) {
                break;
            }
            b = evm_mul(&b, &b);
        }
    }

    result
}

/// EVM SIGNEXTEND: extends the sign bit at byte position b.
/// If b >= 31, returns value unchanged.
#[inline]
pub fn evm_signextend(b: &[u64; 4], value: &[u64; 4]) -> [u64; 4] {
    if b[1] != 0 || b[2] != 0 || b[3] != 0 || b[0] >= 31 {
        return *value;
    }
    let byte_idx = b[0] as usize;
    let bit_idx = byte_idx * 8 + 7;
    let limb_idx = bit_idx / 64;
    let bit_in_limb = bit_idx % 64;

    let sign_bit = (value[limb_idx] >> bit_in_limb) & 1;

    let mut result = *value;
    if sign_bit == 0 {
        result[limb_idx] &= (1u64 << (bit_in_limb + 1)) - 1;
        for i in (limb_idx + 1)..4 {
            result[i] = 0;
        }
    } else {
        result[limb_idx] |= !((1u64 << (bit_in_limb + 1)) - 1);
        for i in (limb_idx + 1)..4 {
            result[i] = u64::MAX;
        }
    }
    result
}

/// EVM LT: a < b (unsigned)
#[inline]
pub fn evm_lt(a: &[u64; 4], b: &[u64; 4]) -> bool {
    lt_u256(a, b)
}

/// EVM GT: a > b (unsigned)
#[inline]
pub fn evm_gt(a: &[u64; 4], b: &[u64; 4]) -> bool {
    lt_u256(b, a)
}

/// EVM SLT: a < b (signed, two's complement)
#[inline]
pub fn evm_slt(a: &[u64; 4], b: &[u64; 4]) -> bool {
    let a_neg = is_negative(a);
    let b_neg = is_negative(b);
    match (a_neg, b_neg) {
        (true, false) => true,
        (false, true) => false,
        _ => lt_u256(a, b),
    }
}

/// EVM SGT: a > b (signed, two's complement)
#[inline]
pub fn evm_sgt(a: &[u64; 4], b: &[u64; 4]) -> bool {
    evm_slt(b, a)
}

/// EVM EQ: a == b
#[inline]
pub fn evm_eq(a: &[u64; 4], b: &[u64; 4]) -> bool {
    a == b
}

/// EVM ISZERO: a == 0
#[inline]
pub fn evm_iszero(a: &[u64; 4]) -> bool {
    is_zero(a)
}

/// EVM AND
#[inline]
pub fn evm_and(a: &[u64; 4], b: &[u64; 4]) -> [u64; 4] {
    [a[0] & b[0], a[1] & b[1], a[2] & b[2], a[3] & b[3]]
}

/// EVM OR
#[inline]
pub fn evm_or(a: &[u64; 4], b: &[u64; 4]) -> [u64; 4] {
    [a[0] | b[0], a[1] | b[1], a[2] | b[2], a[3] | b[3]]
}

/// EVM XOR
#[inline]
pub fn evm_xor(a: &[u64; 4], b: &[u64; 4]) -> [u64; 4] {
    [a[0] ^ b[0], a[1] ^ b[1], a[2] ^ b[2], a[3] ^ b[3]]
}

/// EVM NOT
#[inline]
pub fn evm_not(a: &[u64; 4]) -> [u64; 4] {
    [!a[0], !a[1], !a[2], !a[3]]
}

/// EVM BYTE: get the i-th byte (big-endian indexing, byte 0 = MSB).
/// Returns 0 if i >= 32.
#[inline]
pub fn evm_byte(i: &[u64; 4], value: &[u64; 4]) -> [u64; 4] {
    if i[1] != 0 || i[2] != 0 || i[3] != 0 || i[0] >= 32 {
        return ZERO;
    }
    let idx = i[0] as usize;
    let be_idx = 31 - idx;
    let limb_idx = be_idx / 8;
    let byte_in_limb = be_idx % 8;
    let b = (value[limb_idx] >> (byte_in_limb * 8)) & 0xFF;
    [b, 0, 0, 0]
}

/// EVM SHL: value << shift. Returns 0 if shift >= 256.
#[inline]
pub fn evm_shl(shift: &[u64; 4], value: &[u64; 4]) -> [u64; 4] {
    if shift[1] != 0 || shift[2] != 0 || shift[3] != 0 || shift[0] >= 256 {
        return ZERO;
    }
    let s = shift[0] as u32;
    let limb_shift = (s / 64) as usize;
    let bit_shift = s % 64;

    let mut result = ZERO;
    if bit_shift == 0 {
        for i in limb_shift..4 {
            result[i] = value[i - limb_shift];
        }
    } else {
        for i in limb_shift..4 {
            result[i] = value[i - limb_shift] << bit_shift;
            if i > limb_shift {
                result[i] |= value[i - limb_shift - 1] >> (64 - bit_shift);
            }
        }
    }
    result
}

/// EVM SHR: value >> shift (logical). Returns 0 if shift >= 256.
#[inline]
pub fn evm_shr(shift: &[u64; 4], value: &[u64; 4]) -> [u64; 4] {
    if shift[1] != 0 || shift[2] != 0 || shift[3] != 0 || shift[0] >= 256 {
        return ZERO;
    }
    let s = shift[0] as u32;
    let limb_shift = (s / 64) as usize;
    let bit_shift = s % 64;

    let mut result = ZERO;
    if bit_shift == 0 {
        for i in 0..(4 - limb_shift) {
            result[i] = value[i + limb_shift];
        }
    } else {
        for i in 0..(4 - limb_shift) {
            result[i] = value[i + limb_shift] >> bit_shift;
            if i + limb_shift + 1 < 4 {
                result[i] |= value[i + limb_shift + 1] << (64 - bit_shift);
            }
        }
    }
    result
}

/// EVM SAR: arithmetic shift right. Fills with sign bit.
#[inline]
pub fn evm_sar(shift: &[u64; 4], value: &[u64; 4]) -> [u64; 4] {
    let negative = is_negative(value);
    if shift[1] != 0 || shift[2] != 0 || shift[3] != 0 || shift[0] >= 256 {
        return if negative { MAX } else { ZERO };
    }

    let mut result = evm_shr(shift, value);
    if negative {
        let s = shift[0] as u32;
        if s > 0 {
            let fill_shift = [((256 - s) as u64), 0, 0, 0];
            let mask = evm_shl(&fill_shift, &MAX);
            result = evm_or(&result, &mask);
        }
    }
    result
}
