#![no_std]

#![feature(allocator_api)]
#![feature(associated_type_bounds)]
#![feature(bool_to_option)]
#![feature(const_fn)]
#![feature(core_intrinsics)]
#![feature(const_fn_transmute)]
#![feature(const_mut_refs)]
#![feature(dropck_eyepatch)]

#[cfg(feature = "alloc")]
extern crate alloc;

#[macro_use]
pub mod bitfield;

#[macro_use]
mod macros;

pub mod int;

pub mod intrusive;

pub mod pool;

pub mod spin;

pub mod str;

pub mod stride;

mod volatile;
pub use volatile::*;

pub use chos_lib_macros::forward_fmt;
