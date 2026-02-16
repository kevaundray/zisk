mod fcalls;
#[cfg(not(feature = "guest"))]
mod fcalls_impl;
#[cfg(not(feature = "guest"))]
pub mod lib;

pub use fcalls::*;
#[cfg(not(feature = "guest"))]
pub use fcalls_impl::*;
#[cfg(not(feature = "guest"))]
pub use lib::*;
