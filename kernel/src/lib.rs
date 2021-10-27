#![no_std]
#![feature(allocator_api)]
#![feature(asm)]
#![feature(const_fn_trait_bound)]
#![feature(default_alloc_error_handler)]
#![feature(decl_macro)]
#![feature(int_abs_diff)]
#![feature(maybe_uninit_slice)]
#![feature(negative_impls)]
#![feature(never_type)]
#![feature(option_result_unwrap_unchecked)]
#![feature(ptr_metadata)]
#![feature(thread_local)]
#![warn(clippy::disallowed_method)]

extern crate alloc;

mod early;
mod mm;
mod panic;

fn kernel_main(_: u8) -> ! {
    panic!("Reached kernel_main");
}
