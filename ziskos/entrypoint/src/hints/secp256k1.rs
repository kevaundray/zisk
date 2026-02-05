use crate::hints::macros::define_hint;

const SECP256K1_ECDSA_RECOVER_HINT_ID: u32 = 0x0300;
const SECP256K1_ECDSA_VERIFY_HINT_ID: u32 = 0x0302;

define_hint! {
    secp256k1_ecdsa_address_recover => {
        hint_id: SECP256K1_ECDSA_RECOVER_HINT_ID,
        params: (sig: 64, recid: 8, msg: 32),
        is_result: false,
    }
}

define_hint! {
    secp256k1_ecdsa_verify_and_address_recover => {
        hint_id: SECP256K1_ECDSA_VERIFY_HINT_ID,
        params: (sig: 64, msg: 32, pk: 64),
        is_result: false,
    }
}
