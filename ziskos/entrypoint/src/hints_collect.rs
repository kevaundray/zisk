#[cfg(feature = "hints")]
use std::cell::RefCell;

#[cfg(feature = "hints")]
thread_local! {
    static BUFFER: RefCell<Vec<u64>> = RefCell::new(Vec::with_capacity(64));
}

/// Append a single u64 to the hints buffer.
#[cfg(feature = "hints")]
#[inline(always)]
pub fn hints_push(val: u64) {
    BUFFER.with(|b| b.borrow_mut().push(val));
}

/// Append a slice of u64 values to the hints buffer.
#[cfg(feature = "hints")]
#[inline(always)]
pub fn hints_extend(slice: &[u64]) {
    BUFFER.with(|b| b.borrow_mut().extend_from_slice(slice));
}

/// Drain all collected hints and return them.
#[cfg(feature = "hints")]
#[inline]
pub fn hints_drain() -> Vec<u64> {
    BUFFER.with(|b| std::mem::replace(&mut *b.borrow_mut(), Vec::with_capacity(64)))
}
