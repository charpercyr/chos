mod arc;
use core::alloc::{AllocError, Allocator, Layout};
use core::ptr::NonNull;

pub use arc::*;

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
    ($(pub $(($($vis:tt)*))?)? struct $name:ident => $r:expr) => {
        $(pub $(($($vis)*))*)* struct $name;
        unsafe impl<T> $crate::pool::Pool<T> for $name {
            unsafe fn allocate(
                &self,
            ) -> core::result::Result<core::ptr::NonNull<T>, core::alloc::AllocError> {
                $crate::pool::Pool::<T>::allocate($r)
            }
            unsafe fn deallocate(&self, ptr: core::ptr::NonNull<T>) {
                $crate::pool::Pool::<T>::deallocate($r, ptr)
            }
        }
        impl $crate::init::ConstInit for $name {
            const INIT: Self = Self;
        }
    };
    ($(pub $(($($vis:tt)*))?)? struct $name:ident: $ty:ident => $r:expr) => {
        $(pub $(($($vis)*))*)* struct $name;
        unsafe impl $crate::pool::Pool<$ty> for $name {
            unsafe fn allocate(
                &self,
            ) -> core::result::Result<core::ptr::NonNull<$ty>, core::alloc::AllocError> {
                $crate::pool::Pool::<$ty>::allocate($r)
            }
            unsafe fn deallocate(&self, ptr: core::ptr::NonNull<$ty>) {
                $crate::pool::Pool::<$ty>::deallocate($r, ptr)
            }
        }
        impl $crate::init::ConstInit for $name {
            const INIT: Self = Self;
        }
    };
}
