mod fcalls;
#[cfg(not(target_os = "none"))]
mod fcalls_impl;
#[cfg(not(target_os = "none"))]
pub mod lib;

pub use fcalls::*;
#[cfg(not(target_os = "none"))]
pub use fcalls_impl::*;
#[cfg(not(target_os = "none"))]
pub use lib::*;
