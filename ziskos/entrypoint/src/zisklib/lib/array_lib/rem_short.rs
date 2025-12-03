use crate::zisklib::fcall_division;

use super::{add_short, mul_short, ShortScratch, U256};

/// Division of a large number (represented as an array of U256) by a short U256 number
///
/// It assumes that len(a) > 0, b > 0
pub fn rem_short(a: &[U256], b: &U256, scratch: &mut ShortScratch) -> U256 {
    let len_a = a.len();
    #[cfg(debug_assertions)]
    {
        assert_ne!(len_a, 0, "Input 'a' must have at least one limb");
        assert!(!b.is_zero(), "Input 'b' must be greater than zero");
    }

    if len_a == 1 {
        let a = a[0];
        if a.is_zero() {
            // Return r = 0
            return U256::ZERO;
        } else if a.lt(b) {
            // Return r = a
            return a;
        } else if a.eq(b) {
            // Return r = 0
            return U256::ZERO;
        }
    }
    // We can assume a > b from here on

    // Strategy: Hint the out of the division and then verify it is satisfied
    let a_flat = U256::slice_to_flat(a);

    let (limbs_quo, _) = fcall_division(a_flat, b.as_limbs(), &mut scratch.quo, &mut scratch.rem);
    let quo = U256::flat_to_slice(&scratch.quo[..limbs_quo]);
    let rem = U256::from_u64s(&scratch.rem);

    // The quotient must satisfy 1 <= len(Q) <= len(inA)
    let len_quo = quo.len();
    assert!(len_quo > 0, "Quotient must have at least one limb");
    assert!(len_quo <= len_a, "Quotient length must be less than or equal to dividend length");
    assert!(!quo[len_quo - 1].is_zero(), "Quotient must not have leading zeros");

    // Multiply the quotient by b
    let q_b_len = mul_short(quo, b, &mut scratch.q_b);

    if rem.is_zero() {
        // If the remainder is zero, then we should check that a must be equal to q·b
        assert!(
            U256::eq_slices(a, &scratch.q_b[..q_b_len]),
            "Remainder is zero, but a != q·b\n a = {:?}\n q = {:?}\n b = {:?}\n q·b = {:?}",
            a,
            quo,
            b,
            scratch.q_b,
        );
    } else {
        // If the remainder is non-zero, then we should check that a must be equal to q·b + r and r < b
        assert!(rem.lt(b), "Remainder must be less than divisor");

        let q_b_r_len = add_short(&scratch.q_b[..q_b_len], &rem, &mut scratch.q_b_r);
        assert!(
            U256::eq_slices(a, &scratch.q_b_r[..q_b_r_len]),
            "Remainder is not zero, but a != q·b + r\n a = {:?}\n q = {:?}\n b = {:?}\n r = {:?}\n q·b = {:?}\n q·b+r = {:?}",
            a,
            quo,
            b,
            rem,
            scratch.q_b,
            scratch.q_b_r,
        );
    }

    rem
}
