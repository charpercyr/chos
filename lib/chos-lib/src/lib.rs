#![no_std]
#![feature(bool_to_option)]
#![feature(const_fn)]
#![feature(core_intrinsics)]

#[macro_use]
mod macros;

pub mod int;

pub mod str;

pub mod stride;

mod volatile;
pub use volatile::*;

pub use chos_lib_macros::*;
