#![no_std]
#![no_main]

#![feature(asm)]
#![feature(const_fn_transmute)]
#![feature(global_asm)]

#![feature(custom_test_frameworks)]
#![test_runner(crate::tests::test_runner)]
#![reexport_test_harness_main = "test_main"]

mod arch;

// extern crate alloc;