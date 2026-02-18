use num_bigint::BigUint;

pub static P: spin::Lazy<BigUint> = spin::Lazy::new(|| {
    BigUint::parse_bytes(b"fffffffffffffffffffffffffffffffffffffffffffffffffffffffefffffc2f", 16)
        .unwrap()
});

pub static P_HALF: spin::Lazy<BigUint> = spin::Lazy::new(|| {
    BigUint::parse_bytes(b"7fffffffffffffffffffffffffffffffffffffffffffffffffffffff7ffffe17", 16)
        .unwrap()
});

pub static P_DIV_4: spin::Lazy<BigUint> = spin::Lazy::new(|| {
    BigUint::parse_bytes(b"3fffffffffffffffffffffffffffffffffffffffffffffffffffffffbfffff0c", 16)
        .unwrap()
});

pub static NQR: spin::Lazy<BigUint> = spin::Lazy::new(|| BigUint::from(3u64));

pub static N: spin::Lazy<BigUint> = spin::Lazy::new(|| {
    BigUint::parse_bytes(b"fffffffffffffffffffffffffffffffebaaedce6af48a03bbfd25e8cd0364141", 16)
        .unwrap()
});

pub const IDENTITY: [u64; 8] = [0u64; 8];

pub const G: [u64; 8] = [
    0x59F2815B16F81798,
    0x029BFCDB2DCE28D9,
    0x55A06295CE870B07,
    0x79BE667EF9DCBBAC,
    0x9C47D08FFB10D4B8,
    0xFD17B448A6855419,
    0x5DA4FBFC0E1108A8,
    0x483ADA7726A3C465,
];
