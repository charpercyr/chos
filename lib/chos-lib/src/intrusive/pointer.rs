pub trait PointerOps {
    type Metadata;
    type Target: ?Sized;

    fn into_raw(this: Self) -> (*const Self::Target, Self::Metadata);
    unsafe fn from_raw(ptr: *const Self::Target, meta: Self::Metadata) -> Self;
}

pub unsafe trait ExclusivePointerOps: PointerOps {}

impl<T: ?Sized> PointerOps for &T {
    type Metadata = ();
    type Target = T;

    fn into_raw(this: Self) -> (*const Self::Target, Self::Metadata) {
        (this, ())
    }
    unsafe fn from_raw(ptr: *const Self::Target, _: Self::Metadata) -> Self {
        &*ptr
    }
}

impl<T: ?Sized> PointerOps for &mut T {
    type Metadata = ();
    type Target = T;

    fn into_raw(this: Self) -> (*const Self::Target, Self::Metadata) {
        (this, ())
    }
    unsafe fn from_raw(ptr: *const Self::Target, _: Self::Metadata) -> Self {
        &mut *(ptr as *mut Self::Target)
    }
}

unsafe impl<T: ?Sized> ExclusivePointerOps for &mut T {}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct UnsafeRef<T: ?Sized>(*mut T);
impl<T: ?Sized> UnsafeRef<T> {
    pub unsafe fn new(ptr: *mut T) -> Self {
        Self(ptr)
    }

    pub fn as_ptr(&self) -> *mut T {
        self.0
    }

    pub fn as_ref(&self) -> &T {
        unsafe { &*self.0 }
    }

    pub fn as_mut(&mut self) -> &mut T {
        unsafe { &mut *self.0 }
    }
}
unsafe impl<T: ?Sized + Send> Send for UnsafeRef<T> {}
unsafe impl<T: ?Sized + Sync> Sync for UnsafeRef<T> {}

impl<T: ?Sized> PointerOps for UnsafeRef<T> {
    type Metadata = ();
    type Target = T;

    fn into_raw(this: Self) -> (*const Self::Target, Self::Metadata) {
        (this.as_ptr(), ())
    }
    unsafe fn from_raw(ptr: *const Self::Target, _: Self::Metadata) -> Self {
        Self::new(ptr as _)
    }
}
unsafe impl<T: ?Sized> ExclusivePointerOps for UnsafeRef<T> {}

impl<T: ?Sized> core::ops::Deref for UnsafeRef<T> {
    type Target = T;
    fn deref(&self) -> &Self::Target {
        self.as_ref()
    }
}
impl<T: ?Sized> core::ops::DerefMut for UnsafeRef<T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.as_mut()
    }
}

#[cfg(feature = "alloc")]
mod _alloc {
    use alloc::alloc::Allocator;
    use alloc::boxed::Box;
    use alloc::rc::Rc;
    use alloc::sync::Arc;

    use super::*;

    impl<T: ?Sized, A: Allocator> PointerOps for Box<T, A> {
        type Metadata = A;
        type Target = T;
        fn into_raw(this: Self) -> (*const Self::Target, Self::Metadata) {
            let (ptr, meta) = Self::into_raw_with_allocator(this);
            (ptr, meta)
        }
        unsafe fn from_raw(ptr: *const Self::Target, meta: Self::Metadata) -> Self {
            Self::from_raw_in(ptr as *mut _, meta)
        }
    }
    unsafe impl<T: ?Sized, A: Allocator> ExclusivePointerOps for Box<T, A> {}

    impl<T: ?Sized> PointerOps for Rc<T> {
        type Metadata = ();
        type Target = T;
        fn into_raw(this: Self) -> (*const Self::Target, Self::Metadata) {
            (Self::into_raw(this), ())
        }
        unsafe fn from_raw(ptr: *const Self::Target, _: Self::Metadata) -> Self {
            Self::from_raw(ptr)
        }
    }

    impl<T: ?Sized> PointerOps for Arc<T> {
        type Metadata = ();
        type Target = T;
        fn into_raw(this: Self) -> (*const Self::Target, Self::Metadata) {
            (Self::into_raw(this), ())
        }
        unsafe fn from_raw(ptr: *const Self::Target, _: Self::Metadata) -> Self {
            Self::from_raw(ptr)
        }
    }
}
#[cfg(feature = "alloc")]
pub use _alloc::*;