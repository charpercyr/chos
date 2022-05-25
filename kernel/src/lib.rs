#![no_std]
#![feature(allocator_api)]
#![feature(arbitrary_self_types)]
#![feature(asm_const)]
#![feature(asm_sym)]
#![feature(async_closure)]
#![feature(associated_type_bounds)]
#![feature(bench_black_box)]
#![feature(const_mut_refs)]
#![feature(core_intrinsics)]
#![feature(default_alloc_error_handler)]
#![feature(decl_macro)]
#![feature(downcast_unchecked)]
#![feature(is_some_with)]
#![feature(lang_items)]
#![feature(maybe_uninit_slice)]
#![feature(maybe_uninit_uninit_array)]
#![feature(naked_functions)]
#![feature(negative_impls)]
#![feature(never_type)]
#![feature(new_uninit)]
#![feature(nonnull_slice_from_raw_parts)]
#![feature(ptr_metadata)]
#![feature(poll_ready)]
#![feature(thread_local)]
#![feature(trait_alias)]
#![feature(trait_upcasting)]
#![feature(try_blocks)]
#![feature(vec_into_raw_parts)]
#![cfg_attr(target_arch = "x86_64", feature(abi_x86_interrupt))]
#![allow(improper_ctypes)]
#![warn(clippy::disallowed_method)]
#![allow(incomplete_features)]

extern crate alloc;

pub mod arch;
pub mod async_;
pub mod config;
pub mod cpumask;
pub mod driver;
mod dummy;
mod early;
pub mod fs;
mod initrd;
pub mod intr;
mod kmain;
pub mod mm;
pub mod module;
mod panic;
pub mod resource;
pub mod sched;
mod symbols;
pub mod timer;
pub mod util;

pub extern crate paste;
