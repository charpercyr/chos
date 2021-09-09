#![no_std]
#![feature(allocator_api)]
#![feature(associated_type_bounds)]
#![feature(bool_to_option)]
#![feature(core_intrinsics)]
#![feature(const_fn_trait_bound)]
#![feature(const_mut_refs)]
#![feature(const_panic)]
#![feature(const_unreachable_unchecked)]
#![feature(decl_macro)]
#![feature(dropck_eyepatch)]
#![feature(negative_impls)]

mod macros;
pub use macros::*;

mod either;
pub use either::*;

pub mod int;

pub mod intrusive;

pub mod iter;

pub mod log;

pub mod mm;

pub mod pool;

pub mod str;

pub mod stride;

pub mod sync;

mod volatile;
pub use chos_lib_macros::forward_fmt;
pub use volatile::*;

#[cfg(any(test, feature = "alloc"))]
extern crate alloc;

#[cfg(test)]
extern crate std;

pub use cfg_if;
