
mod arc;
pub use arc::*;

use core::alloc::{AllocError, Allocator, Layout};
use core::ptr::NonNull;

pub unsafe trait Pool<T> {
    unsafe fn allocate(&self) -> Result<NonNull<T>, AllocError>;
    unsafe fn deallocate(&self, ptr: NonNull<T>);
}

unsafe impl<A: Allocator, T> Pool<T> for A {
    unsafe fn allocate(&self) -> Result<NonNull<T>, AllocError> {
        A::allocate(self, Layout::new::<T>()).map(|ptr| ptr.cast())
    }
    unsafe fn deallocate(&self, ptr: NonNull<T>) {
        A::deallocate(self, ptr.cast(), Layout::new::<T>())
    }
}

#[macro_export]
macro_rules! pool {
    ($name:ident => $r:expr) => {
        pub struct $name;
        unsafe impl<T> $crate::pool::Pool<T> for $name {
            unsafe fn allocate(&self) -> Result<NonNull<T>, AllocError> {
                $crate::pool::Pool::<T>::allocate($r)
            }
            unsafe fn deallocate(&self, ptr: NonNull<T>) {
                $crate::pool::Pool::<T>::deallocate($r, ptr)
            }
        }
    };
}
