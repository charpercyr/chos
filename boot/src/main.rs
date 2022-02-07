#![no_std]
#![no_main]
#![allow(incomplete_features)]
#![feature(abi_x86_interrupt)]
#![feature(decl_macro)]
#![feature(fn_traits)]
#![feature(inline_const)]
#![feature(never_type)]
#![feature(once_cell)]
#![feature(ptr_metadata)]
#![feature(unboxed_closures)]

mod arch;

#[no_mangle]
fn __disable_sched_save() -> u64 {
    0
}
#[no_mangle]
fn __restore_sched(_: u64) {}
