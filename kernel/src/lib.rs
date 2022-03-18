#![no_std]
#![feature(allocator_api)]
#![feature(asm_const)]
#![feature(associated_type_bounds)]
#![feature(bench_black_box)]
#![feature(bool_to_option)]
#![feature(const_mut_refs)]
#![feature(core_intrinsics)]
#![feature(default_alloc_error_handler)]
#![feature(decl_macro)]
#![feature(is_some_with)]
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
mod config;
mod cpumask;
mod dummy;
mod early;
mod fs;
mod intr;
mod kmain;
mod log;
mod mm;
mod panic;
mod sched;
mod symbols;
mod timer;
mod util;
