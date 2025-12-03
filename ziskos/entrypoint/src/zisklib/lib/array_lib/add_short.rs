use crate::syscalls::{syscall_add256, SyscallAdd256Params};

use super::U256;

/// Addition of one large number (represented as an array of U256) and a short U256 number
///
/// It assumes that a,b > 0
pub fn add_short(a: &[U256], b: &U256, out: &mut [U256]) -> usize {
    let len_a = a.len();
    #[cfg(debug_assertions)]
    {
        assert_ne!(len_a, 0, "Input 'a' must have at least one limb");
        assert!(!a[len_a - 1].is_zero(), "Input 'a' must not have leading zeros");
        assert!(!b.is_zero(), "Input 'b' must be greater than zero");
    }

    // Start with a[0] + b
    let mut params = SyscallAdd256Params {
        a: a[0].as_limbs(),
        b: b.as_limbs(),
        cin: 0,
        c: out[0].as_limbs_mut(),
    };
    let mut carry = syscall_add256(&mut params);

    for i in 1..len_a {
        if carry == 1 {
            // Compute a[i] + carry
            let mut params = SyscallAdd256Params {
                a: a[i].as_limbs(),
                b: U256::ZERO.as_limbs(),
                cin: 1,
                c: out[i].as_limbs_mut(),
            };
            carry = syscall_add256(&mut params);
        } else {
            // Directly copy a[i] to out[i]
            out[i] = a[i];
        }
    }

    if carry == 0 {
        len_a
    } else {
        out[len_a] = U256::ONE;
        len_a + 1
    }
}
