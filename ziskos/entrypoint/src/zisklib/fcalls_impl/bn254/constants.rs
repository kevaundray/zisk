use num_bigint::BigUint;

pub(crate) static P: spin::Lazy<BigUint> = spin::Lazy::new(|| {
    BigUint::parse_bytes(
        b"30644e72e131a029b85045b68181585d97816a916871ca8d3c208c16d87cfd47",
        16,
    )
    .unwrap()
});
