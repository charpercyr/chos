mod arc;
mod boxed;
#[cfg(feature = "alloc")]
use alloc::alloc::Global;
use core::alloc::{AllocError, Allocator, Layout};
use core::ptr::NonNull;

pub use arc::*;
pub use boxed::*;

use crate::init::ConstInit;

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

#[cfg(feature = "alloc")]
pub fn handle_alloc_error(layout: Layout) -> ! {
    alloc::alloc::handle_alloc_error(layout)
}
#[cfg(not(feature = "alloc"))]
pub fn handle_alloc_error(layout: Layout) -> ! {
    panic!(
        "Could not allocate (size={}, align={})",
        layout.size(),
        layout.align()
    )
}

pub unsafe trait ConstPool<T>: Pool<T> + ConstInit + Copy {}

pub trait ConstPoolExt<T>: ConstPool<T> {
    fn try_boxed(value: T) -> Result<PoolBox<T, Self>, AllocError> {
        PoolBox::try_new_in(value, Self::INIT)
    }
    fn boxed(value: T) -> PoolBox<T, Self> {
        PoolBox::new_in(value, Self::INIT)
    }
    fn try_arc(value: T) -> Result<IArc<T, Self>, AllocError> where T: IArcAdapter {
        IArc::try_new_in(value, Self::INIT)
    }
    fn arc(value: T) -> IArc<T, Self> where T: IArcAdapter {
        IArc::new_in(value, Self::INIT)
    }
}
impl<T, P: ConstPool<T>> ConstPoolExt<T> for P {}

#[cfg(feature = "alloc")]
unsafe impl<T> ConstPool<T> for Global {}

#[macro_export]
macro_rules! pool {
    ($(pub $(($($vis:tt)*))?)? struct $name:ident => $r:expr) => {
        #[derive(Debug, Clone, Copy, PartialEq, Eq)]
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
        unsafe impl<T> $crate::pool::ConstPool<T> for $name {}
    };
    ($(pub $(($($vis:tt)*))?)? struct $name:ident: $ty:ident => $r:expr) => {
        #[derive(Debug, Clone, Copy, PartialEq, Eq)]
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
        unsafe impl $crate::pool::ConstPool<$ty> for $name {}
    };
}
