#![no_std]
#![feature(allocator_api)]
#![feature(asm)]
#![feature(associated_type_bounds)]
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
#![warn(clippy::disallowed_method)]
// #![warn(unsafe_op_in_unsafe_fn)]

extern crate alloc;

mod arch;
mod dummy;
mod early;
mod mm;
mod panic;

fn kernel_main(_: u8) -> ! {
    panic!("Reached kernel_main");
}
