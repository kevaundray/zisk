#[cfg(feature = "hints")]
use std::cell::{Cell, RefCell};

#[cfg(feature = "hints")]
thread_local! {
    static BUFFER: RefCell<Vec<u64>> = RefCell::new(Vec::with_capacity(64));
    static DEPTH: Cell<u32> = Cell::new(0);
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

/// Clear the hints buffer and begin a collection scope.
/// Panics if called while another scope is active (reentrant call).
#[cfg(feature = "hints")]
#[inline]
pub fn hints_clear() {
    DEPTH.with(|d| {
        assert_eq!(
            d.get(),
            0,
            "hints_clear: reentrant call detected — nested handler invocations are not supported"
        );
        d.set(1);
    });
    BUFFER.with(|b| b.borrow_mut().clear());
}

/// Drain all collected hints, end the collection scope, and return them.
#[cfg(feature = "hints")]
#[inline]
pub fn hints_drain() -> Vec<u64> {
    DEPTH.with(|d| d.set(0));
    BUFFER.with(|b| std::mem::replace(&mut *b.borrow_mut(), Vec::with_capacity(64)))
}
