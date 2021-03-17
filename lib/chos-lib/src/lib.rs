#![no_std]

#![feature(allocator_api)]
#![feature(bool_to_option)]
#![feature(const_fn)]
#![feature(core_intrinsics)]
#![feature(dropck_eyepatch)]

#[cfg(feature = "alloc")]
extern crate alloc;

#[macro_use]
mod macros;

pub mod int;

pub mod intrusive;

pub mod pool;

pub mod str;

pub mod stride;

mod volatile;
pub use volatile::*;

pub use chos_lib_macros::*;
