use std::cmp::Ordering;

use crate::zisklib::fcall_division;

use super::{add_short, mul_short, U256};

/// Division of a large number (represented as an array of U256) by a short U256 number
///
/// It assumes that len(a) > 0, b > 1
pub fn div_short(a: &[U256], b: &U256) -> (Vec<U256>, U256) {
    let len_a = a.len();
    #[cfg(debug_assertions)]
    {
        assert_ne!(len_a, 0, "Input 'a' must have at least one limb");
        assert!(b.gt(&U256::ONE), "Input 'b' must be greater than one");
    }

    if len_a == 1 {
        let a = a[0];
        if a.is_zero() {
            // Return q = 0, r = 0
            return (vec![U256::ZERO], U256::ZERO);
        }

        // Check whether a < b or a == b
        if a.lt(b) {
            // Return q = 0, r = a
            return (vec![U256::ZERO], a);
        } else if a.eq(b) {
            // Return q = 1, r = 0
            return (vec![U256::ONE], U256::ZERO);
        }
    }

    // Check if a = b, a < b or a > b
    let comp = U256::compare_slices(a, &[*b]);
    if comp == Ordering::Less {
        // a < b. Return q = 0, r = a
        return (vec![U256::ZERO], a[0]);
    } else if comp == Ordering::Equal {
        // a == b. Return q = 1, r = 0
        return (vec![U256::ONE], U256::ZERO);
    }

    // We can assume a > b from here on

    // Strategy: Hint the out of the division and then verify it is satisfied
    let a_flat = U256::slice_to_flat(a);

    let max_quo_len = len_a * 4;
    let mut quo_flat = vec![0u64; max_quo_len];
    let mut rem_flat = vec![0u64; 4];
    let (len_quo, len_rem) = fcall_division(a_flat, b.as_limbs(), &mut quo_flat, &mut rem_flat);
    let quo = U256::slice_from_flat(&quo_flat[..len_quo]);
    let rem = U256::slice_from_flat(&rem_flat[..len_rem])[0];

    // The quotient must satisfy 1 <= len(Q) <= len(inA)
    let len_quo = quo.len();
    assert!(len_quo > 0, "Quotient must have at least one limb");
    assert!(len_quo <= len_a, "Quotient length must be less than or equal to dividend length");
    assert!(!quo[len_quo - 1].is_zero(), "Quotient must not have leading zeros");

    // Multiply the quotient by b
    let q_b = mul_short(quo, b);

    if rem.is_zero() {
        // If the remainder is zero, then a must be equal to q·b
        assert!(U256::eq_slices(a, &q_b), "Remainder is zero, but a != q·b");
    } else {
        // If the remainder is non-zero, then a must be equal to q·b + r and r < b
        assert!(!rem.is_zero(), "Remainder must be non-zero");
        assert!(rem.lt(b), "Remainder must be less than divisor");

        let q_b_r = add_short(&q_b, &rem);
        assert!(U256::eq_slices(a, &q_b_r), "a != q·b + r");
    }

    (quo.to_vec(), rem)
}
