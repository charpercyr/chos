
mod arc;
pub use arc::*;

use core::alloc::{AllocError, Allocator, Layout};
use core::ptr::NonNull;

#[cfg(feature = "alloc")]
use alloc::alloc::Global;

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

pub trait GlobalPool<T>: Pool<T> {
    const VALUE: Self;

    fn handle_alloc_error() -> !;
}

#[cfg(feature = "alloc")]
impl<T> GlobalPool<T> for Global {
    const VALUE: Self = Global;

    fn handle_alloc_error() -> ! {
        handle_alloc_error(Layout::new::<T>());
    }
}

#[cfg(feature = "alloc")]
pub fn handle_alloc_error(layout: Layout) -> ! {
    alloc::alloc::handle_alloc_error(layout)
}

#[cfg(not(feature = "alloc"))]
pub fn handle_alloc_error(layout: Layout) -> ! {
    panic!("Could not allocate {:?}", layout)
}

#[macro_export]
macro_rules! global_pool {
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

        impl<T> $crate::pool::GlobalPool<T> for $name {
            const VALUE: Self = Self;
            fn handle_alloc_error() -> ! {
                $crate::pool::handle_alloc_error(core::alloc::Layout::new::<T>());
            }
        }
    };
}
