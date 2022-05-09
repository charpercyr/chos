#![no_std]
#![allow(incomplete_features)]
#![allow(macro_expanded_macro_exports_accessed_by_absolute_paths)]
#![feature(allocator_api)]
#![feature(associated_type_bounds)]
#![feature(associated_type_defaults)]
#![feature(build_hasher_simple_hash_one)]
#![feature(coerce_unsized)]
#![feature(core_intrinsics)]
#![feature(const_mut_refs)]
#![feature(decl_macro)]
#![feature(dropck_eyepatch)]
#![feature(generic_associated_types)]
#![feature(generic_const_exprs)]
#![feature(inherent_associated_types)]
#![feature(layout_for_ptr)]
#![feature(maybe_uninit_uninit_array)]
#![feature(negative_impls)]
#![feature(never_type)]
#![feature(ptr_metadata)]
#![feature(unsize)]
#![feature(untagged_unions)]
#![cfg_attr(
    any(target_arch = "x86", target_arch = "x86_64"),
    feature(abi_x86_interrupt)
)]
#![cfg_attr(feature = "alloc", feature(new_uninit))]

pub mod access;
pub mod arch;
pub mod array;
pub mod boot;
mod config;
pub mod cpumask;
pub mod elf;
pub mod fmt;
pub mod init;
pub mod int;
pub mod intrusive;
pub mod log;
mod macros;
pub mod mm;
pub mod pool;
pub mod ptr;
pub mod queue;
pub mod str;
pub mod stride;
pub mod sync;
pub mod tar;
mod volatile;
pub use chos_lib_macros::forward_fmt;
pub use volatile::*;

#[cfg(any(test, feature = "alloc"))]
extern crate alloc;

#[cfg(test)]
extern crate std;

pub use cfg_if;

#[cfg(test)]
#[no_mangle]
extern "C" fn __lock_disable_sched_save() {}

#[cfg(test)]
#[no_mangle]
extern "C" fn __lock_restore_sched() {}
