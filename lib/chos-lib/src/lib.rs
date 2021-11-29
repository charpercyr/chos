#![no_std]
#![allow(incomplete_features)]
#![feature(allocator_api)]
#![feature(asm)]
#![feature(associated_type_bounds)]
#![feature(bool_to_option)]
#![feature(build_hasher_simple_hash_one)]
#![feature(core_intrinsics)]
#![feature(const_fn_trait_bound)]
#![feature(const_mut_refs)]
#![feature(decl_macro)]
#![feature(dropck_eyepatch)]
#![feature(generic_associated_types)]
#![feature(inherent_associated_types)]
#![feature(maybe_uninit_extra)]
#![feature(maybe_uninit_uninit_array)]
#![feature(negative_impls)]
#![feature(never_type)]
#![feature(ptr_metadata)]

pub mod arch;

pub mod boot;

mod macros;

pub mod elf;

pub mod int;

pub mod init;

pub mod intrusive;

pub mod iter;

pub mod log;

pub mod mm;

pub mod pool;

pub mod ptr;

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
