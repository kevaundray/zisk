use std::cmp::Ordering;

use crate::zisklib::fcall_division;

use super::{add_agtb, mul_long, U256};

/// Division of two large numbers (represented as arrays of U256)
///
/// It assumes that len(a) > 0 and len(b) > 1
pub fn rem_long(a: &[U256], b: &[U256]) -> Vec<U256> {
    let len_a = a.len();
    let len_b = b.len();
    #[cfg(debug_assertions)]
    {
        assert_ne!(len_a, 0, "Input 'a' must have at least one limb");
        assert!(len_b > 1, "Input 'b' must have more than one limb");
    }

    if len_a == 1 {
        let a0 = a[0];
        if a0.is_zero() {
            // Return r = 0
            return vec![U256::ZERO];
        }

        // As len(b) > 1, we have a < b. Return r = a
        return a.to_vec();
    } else if len_a < len_b {
        // We have a < b. Return r = a
        return a.to_vec();
    }

    // Check if a = b, a < b or a > b
    let comp = U256::compare_slices(a, b);
    if comp == Ordering::Less {
        // a < b. Return r = a
        return a.to_vec();
    } else if comp == Ordering::Equal {
        // a == b. Return r = 0
        return vec![U256::ZERO];
    }

    // We can assume a > b from here on

    // Strategy: Hint the out of the division and then verify it is satisfied
    let a_flat = U256::slice_to_flat(a);
    let b_flat = U256::slice_to_flat(b);

    let max_quo_len = (len_a - len_b + 1) * 4;
    let max_rem_len = len_b * 4;
    let mut quo_flat = vec![0u64; max_quo_len];
    let mut rem_flat = vec![0u64; max_rem_len];
    let (len_quo, len_rem) = fcall_division(a_flat, b_flat, &mut quo_flat, &mut rem_flat);
    let quo = U256::slice_from_flat(&quo_flat[..len_quo]);
    let rem = U256::slice_from_flat(&rem_flat[..len_rem]);

    // Since len(a) >= len(b), the division a = q·b + r must satisfy:
    //      1] max{len(q·b), len(r)} <= len(a) => len(q) + len(b) - 1 <= len(q·b) <= len(a)
    //                                         =>                        len(r)   <= len(a)
    //      2] 1 <= len(r) <= len(b)

    // Check 1 <= len(q) <= len(a) - len(b) + 1
    let len_quo = quo.len();
    assert!(len_quo > 0, "Quotient must have at least one limb");
    assert!(
        len_quo <= len_a - len_b + 1,
        "Quotient length must be less than or equal to dividend length"
    );
    assert!(!quo[len_quo - 1].is_zero(), "Quotient must not have leading zeros");

    // Multiply the quotient by b
    let q_b = mul_long(quo, b);

    // Check 1 <= len(r)
    let len_rem = rem.len();
    assert!(len_rem > 0, "Remainder must have at least one limb");

    if len_rem == 1 && rem[0].is_zero() {
        // If the remainder is zero, then a must be equal to q·b
        assert!(U256::eq_slices(a, &q_b), "Remainder is zero, but a != q·b");
    } else {
        // If the remainder is non-zero, check len(r) <= len(b)
        assert!(len_rem <= len_b, "Remainder length must be less than or equal to divisor length");
        assert!(!rem[len_rem - 1].is_zero(), "Remainder must not have leading zeros");

        // We also must have r < b
        assert!(U256::lt_slices(rem, b), "Remainder must be less than divisor");

        // As the remainder is non-zero, then a must be equal to q·b + r
        let q_b_r = add_agtb(&q_b, rem);
        assert!(U256::eq_slices(a, &q_b_r), "a != q·b + r");
    }

    rem.to_vec()
}
