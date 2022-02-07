#![no_std]
#![feature(decl_macro)]

pub mod arch;

pub mod timer {
    pub const TICKS_HZ: u64 = 500;
}
