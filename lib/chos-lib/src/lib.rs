#![no_std]

#![feature(bool_to_option)]
#![feature(const_fn)]
#![feature(core_intrinsics)]

#[macro_use]
mod macros;

pub mod int;

pub mod spin;

pub mod str;

mod volatile;
pub use volatile::*;

pub use chos_lib_macros::*;
