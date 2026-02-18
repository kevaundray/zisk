mod fcalls;
#[cfg(not(feature = "zisk_guest"))]
mod fcalls_impl;
pub mod lib;

pub use fcalls::*;
#[cfg(not(feature = "zisk_guest"))]
pub use fcalls_impl::*;
pub use lib::*;
