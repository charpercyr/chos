#![no_std]
#![feature(allocator_api)]
#![feature(associated_type_bounds)]
#![feature(bool_to_option)]
#![feature(const_fn_fn_ptr_basics)]
#![feature(const_fn_trait_bound)]
#![feature(const_mut_refs)]
#![feature(default_alloc_error_handler)]
#![feature(decl_macro)]
#![feature(int_abs_diff)]
#![feature(lang_items)]
#![feature(maybe_uninit_slice)]
#![feature(negative_impls)]
#![feature(never_type)]
#![feature(new_uninit)]
#![feature(ptr_metadata)]
#![feature(thread_local)]
#![cfg_attr(target_arch = "x86_64", feature(abi_x86_interrupt))]

#![allow(improper_ctypes)]
#![warn(clippy::disallowed_method)]

extern crate alloc;

mod arch;
mod dummy;
mod early;
mod kmain;
mod mm;
mod panic;
