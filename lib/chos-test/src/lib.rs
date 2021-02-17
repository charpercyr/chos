#![no_std]

pub use chos_test_macros::*;

use core::slice::from_raw_parts;

#[repr(C, align(16))]
pub struct TestCase {
    pub name: &'static str,
    pub fun: fn(),
}

pub unsafe fn get_tests<'a>(start: *const TestCase, end: *const TestCase) -> &'a [TestCase] {
    let len = end.offset_from(start) as usize;
    from_raw_parts(start, len)
}
