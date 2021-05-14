#![no_std]

#![feature(allocator_api)]
#![feature(associated_type_bounds)]
#![feature(bool_to_option)]
#![feature(core_intrinsics)]
#![feature(const_fn_transmute)]
#![feature(const_mut_refs)]
#![feature(decl_macro)]
#![feature(dropck_eyepatch)]

#[cfg(feature = "alloc")]
extern crate alloc;

pub mod bitfield;

mod macros;
pub use macros::*;

mod either;
pub use either::*;

pub mod int;

pub mod intrusive;

pub mod pool;

pub mod spin;

pub mod str;

pub mod stride;

mod volatile;
pub use volatile::*;

pub use chos_lib_macros::forward_fmt;
