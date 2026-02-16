macro_rules! define_hint {
    (
        $name:ident => {
            hint_id: $hint_id:expr,
            params: ( $( $arg:ident : $len:literal ),+ $(,)? ),
            is_result: $is_result:expr,
        }
    ) => {
        paste::paste! {
            #[no_mangle]
            pub unsafe extern "C" fn [<hint_ $name>]($( $arg: *const u8 ),+) {
                if !crate::hints::HINT_BUFFER.is_enabled() {
                    return;
                }

                let segs: &[*const u8] = &[$( $arg ),+];
                let lens: &[usize] = &[$( $len ),+];

                crate::hints::HINT_BUFFER.write_hint_segments(
                    $hint_id,
                    segs,
                    lens,
                    $is_result,
                );
            }

            $crate::hints::macros::register_hint_meta!($name, $hint_id);
        }
    };
}

macro_rules! define_hint_pairs {
    (
        $name:ident => {
            hint_id: $hint_id:expr,
            pair_len: $pair_len:expr,
            is_result: $is_result:expr,
        }
    ) => {
        paste::paste! {
            #[no_mangle]
            pub unsafe extern "C" fn [<hint_ $name>](pairs: *const u8, num_pairs: usize) {
                if !crate::hints::HINT_BUFFER.is_enabled() {
                    return;
                }

                let num_pairs_bytes: [u8; 8] = (num_pairs as u64).to_le_bytes();
                let pairs_len = num_pairs * ($pair_len as usize);

                let segs: &[*const u8] = &[num_pairs_bytes.as_ptr(), pairs];
                let lens: &[usize] = &[num_pairs_bytes.len(), pairs_len];

                crate::hints::HINT_BUFFER.write_hint_segments(
                    $hint_id,
                    segs,
                    lens,
                    $is_result,
                );
            }

            $crate::hints::macros::register_hint_meta!($name, $hint_id);
        }
    };
}

macro_rules! define_hint_ptr {
    (
        $name:ident => {
            hint_id: $hint_id:expr,
            param: $arg:ident,
            is_result: $is_result:expr,
        }
    ) => {
        paste::paste! {
            #[no_mangle]
            pub unsafe extern "C" fn [<hint_ $name>](
                [<$arg _ptr>]: *const u8,
                [<$arg _len>]: usize
            ) {
                if !crate::hints::HINT_BUFFER.is_enabled() {
                    return;
                }

                let segs: &[*const u8] = &[ [<$arg _ptr>] ];
                let lens: &[usize] = &[ [<$arg _len>] ];

                crate::hints::HINT_BUFFER.write_hint_segments(
                    $hint_id,
                    segs,
                    lens,
                    $is_result,
                );
            }

            $crate::hints::macros::register_hint_meta!($name, $hint_id);
        }
    };

    (
        $name:ident => {
            hint_id: $hint_id:expr,
            params: ( $( $arg:ident ),+ $(,)? ),
            is_result: $is_result:expr,
        }
    ) => {
        paste::paste! {
            #[no_mangle]
            pub unsafe extern "C" fn [<hint_ $name>](
                $( [<$arg _ptr>]: *const u8, [<$arg _len>]: usize ),+
            ) {
                if !crate::hints::HINT_BUFFER.is_enabled() {
                    return;
                }

                let segs: &[*const u8] = &[$( [<$arg _ptr>] ),+];
                let lens: &[usize] = &[$( [<$arg _len>] ),+];

                crate::hints::HINT_BUFFER.write_hint_len_prefixed_segments(
                    $hint_id,
                    segs,
                    lens,
                    $is_result,
                );
            }

            $crate::hints::macros::register_hint_meta!($name, $hint_id);
        }
    };
}

macro_rules! register_hint_meta {
    ($name:ident, $hint_id:expr) => {
        paste::paste! {
            #[cfg(zisk_hints_metrics)]
            #[ctor::ctor]
            fn [<$name _register_meta>]() {
                $crate::hints::metrics::register_hint($hint_id, stringify!($name).to_string());
            }
        }
    };
}

pub(crate) use define_hint;
pub(crate) use define_hint_pairs;
pub(crate) use define_hint_ptr;
pub(crate) use register_hint_meta;
