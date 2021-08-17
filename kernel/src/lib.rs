#![no_std]
#![feature(allocator_api)]
#![feature(asm)]
#![feature(const_fn_trait_bound)]
#![feature(const_fn_transmute)]
#![feature(default_alloc_error_handler)]
#![feature(decl_macro)]
#![feature(maybe_uninit_slice)]
#![feature(negative_impls)]
#![feature(never_type)]
#![feature(option_result_unwrap_unchecked)]
#![feature(thread_local)]

extern crate alloc;

mod arch;
mod early;
mod log;
mod mm;
mod panic;

fn kernel_main(id: u8) -> ! {
    loop {}
}
