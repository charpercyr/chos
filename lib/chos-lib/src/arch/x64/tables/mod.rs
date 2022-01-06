mod gdt;
mod idt;
mod tss;

use core::{mem::size_of};

pub use gdt::*;
pub use idt::*;
pub use tss::*;

#[repr(C, packed)]
struct DescriptorRegister<T: 'static> {
    pub len: u16,
    pub ptr: &'static T,
}

impl<T: 'static> DescriptorRegister<T> {
    pub const fn new(ptr: &'static T) -> Self {
        unsafe { Self::new_unchecked(ptr) }
    }

    pub const unsafe fn new_unchecked(ptr: *const T) -> Self {
        Self {
            len: (size_of::<T>() - 1) as u16,
            ptr: &*ptr,
        }
    }
}
