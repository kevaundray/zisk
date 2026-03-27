use super::{KeccakStateBits, RC_BITS, RHO_OFFSETS};

// The maximum value that any expression during keccakf computation can get
// Operation summary:
//  - The θ.1 step has 4 add, this gives a number in the range <= 5
//  - The θ.2 step has 1 add, this gives a number in the range <= 10
//  - The θ.3 step has 1 add, this gives a number in the range <= 11
//  - The χ.1 step has 1 add and 1 prod, this gives a number in the range <= 132
//  - The χ.2 step has 1 add, this gives a number in the range <= 143
//  - The ι step has 1 add, this gives a number in the range <= 144
pub fn keccak_f_round(state: &mut KeccakStateBits, round: usize) {
    // θ (Theta) step - Column parity computation and mixing
    theta(state);

    // ρ (Rho) step - Bitwise rotation
    rho(state);

    // π (Pi) step - Lane permutation
    pi(state);

    // χ (Chi) step - Nonlinear transformation
    chi(state);

    // ι (Iota) step - Add round constant
    iota(state, round);

    // Reduce the state modulo 2
    reduce_state_mod2(state);
}

/// θ (Theta) step: For all pairs (x, z) such that 0 ≤ x < 5 and 0 ≤ z < 64:
/// 1. C[x, z] = A[x, 0, z] ⊕ A[x, 1, z] ⊕ A[x, 2, z] ⊕ A[x, 3, z] ⊕ A[x, 4, z]
/// 2. D[x, z] = C[(x-1) mod 5, z] ⊕ C[(x+1) mod 5, (z-1) mod 64]
/// 3. A[x, y, z] = A[x, y, z] ⊕ D[x, z]
#[allow(clippy::needless_range_loop)]
fn theta(state: &mut KeccakStateBits) {
    let mut c = [[0u64; 64]; 5];

    // Step 1: Compute column parities
    for x in 0..5 {
        for z in 0..64 {
            c[x][z] =
                state[x][0][z] + state[x][1][z] + state[x][2][z] + state[x][3][z] + state[x][4][z];
        }
    }

    // Step 2: Compute D[x, z]
    let mut d = [[0u64; 64]; 5];
    for x in 0..5 {
        for z in 0..64 {
            d[x][z] = c[(x + 4) % 5][z] + c[(x + 1) % 5][(z + 63) % 64];
        }
    }

    // Step 3: Apply theta transformation
    for x in 0..5 {
        for y in 0..5 {
            for z in 0..64 {
                state[x][y][z] += d[x][z];
            }
        }
    }
}

/// ρ (Rho) step: For all z such that 0 ≤ z < 64:
/// 1. R[0, 0, z] = A[0, 0, z] (no rotation for [0,0])
/// 2. For other positions, rotate according to RHO_OFFSETS
fn rho(state: &mut KeccakStateBits) {
    let mut temp_state = [[[0u64; 64]; 5]; 5];

    for x in 0..5 {
        for y in 0..5 {
            let rotation = RHO_OFFSETS[x][y];
            if rotation == 0 {
                // No rotation for position [0][0]
                temp_state[x][y] = state[x][y];
            } else {
                // Apply rotation: R[x, y, z] = A[x, y, (z - rotation) mod 64]
                for z in 0..64 {
                    temp_state[x][y][z] = state[x][y][(z + 64 - rotation) % 64];
                }
            }
        }
    }

    *state = temp_state;
}

/// π (Pi) step: For all triples (x, y, z) such that 0 ≤ x,y < 5, and 0 ≤ z < 64:
/// B[x, y, z] = R[(x + 3y) mod 5, x, z]
fn pi(state: &mut KeccakStateBits) {
    let mut temp_state = [[[0u64; 64]; 5]; 5];

    for x in 0..5 {
        for y in 0..5 {
            for z in 0..64 {
                temp_state[x][y][z] = state[(x + 3 * y) % 5][x][z];
            }
        }
    }

    *state = temp_state;
}

/// χ (Chi) step: For all triples (x, y, z) such that 0 ≤ x,y < 5 and 0 ≤ z < 64:
/// A[x, y, z] = B[x, y, z] ⊕ ((¬B[(x + 1) mod 5, y, z]) ∧ B[(x + 2) mod 5, y, z])
fn chi(state: &mut KeccakStateBits) {
    let mut temp_state = [[[0u64; 64]; 5]; 5];

    for x in 0..5 {
        for y in 0..5 {
            for z in 0..64 {
                let b1 = state[(x + 1) % 5][y][z];
                let b2 = state[(x + 2) % 5][y][z];
                temp_state[x][y][z] = state[x][y][z] + ((1 + b1) * b2);
            }
        }
    }

    *state = temp_state;
}

/// ι (Iota) step: For all z such that 0 ≤ z < 64:
/// A[0, 0, z] = A[0, 0, z] ⊕ RC[round][z]
fn iota(state: &mut KeccakStateBits, round: usize) {
    for z in 0..64 {
        state[0][0][z] += RC_BITS[round][z] as u64;
    }
}

/// Reduce the state modulo 2 by applying modulo 2 to each bit in the state
pub(crate) fn reduce_state_mod2(state: &mut KeccakStateBits) {
    state.iter_mut().flatten().flatten().for_each(|bit| *bit %= 2);
}
