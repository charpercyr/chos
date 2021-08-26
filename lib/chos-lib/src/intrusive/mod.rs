pub mod list;

pub trait Adapter {
    type Value: ?Sized;
    type Link;
    type Pointer: Pointer<Target = Self::Value>;

    unsafe fn get_link(&self, value: *const Self::Value) -> *const Self::Link;
    unsafe fn get_value(&self, link: *const Self::Link) -> *const Self::Value;
}

pub trait KeyAdapter<'a>: Adapter {
    type Key;
    fn get_key(&self, value: &'a Self::Value) -> Self::Key;
}

pub trait LinkOps {
    type Metadata;

    fn acquire(&self) -> bool;
    fn release(&self);

    unsafe fn set_meta(&self, meta: Self::Metadata);
    unsafe fn take_meta(&self) -> Self::Metadata;
}

pub trait Pointer {
    type Metadata;
    type Target: ?Sized;

    fn into_raw(this: Self) -> (*const Self::Target, Self::Metadata);
    unsafe fn from_raw(ptr: *const Self::Target, meta: Self::Metadata) -> Self;
}

impl<T: ?Sized> Pointer for &T {
    type Metadata = ();
    type Target = T;

    fn into_raw(this: Self) -> (*const Self::Target, Self::Metadata) {
        (this, ())
    }
    unsafe fn from_raw(ptr: *const Self::Target, _: Self::Metadata) -> Self {
        &*ptr
    }
}

impl<T: ?Sized> Pointer for &mut T {
    type Metadata = ();
    type Target = T;

    fn into_raw(this: Self) -> (*const Self::Target, Self::Metadata) {
        (this, ())
    }
    unsafe fn from_raw(ptr: *const Self::Target, _: Self::Metadata) -> Self {
        &mut *(ptr as *mut Self::Target)
    }
}

pub struct UnsafeRef<T: ?Sized>(*const T);
impl<T: ?Sized> UnsafeRef<T> {
    pub unsafe fn new(ptr: *const T) -> Self {
        Self(ptr)
    }

    pub fn as_ptr(&self) -> *const T {
        self.0
    }

    pub fn as_ref(&self) -> &T {
        unsafe { &*self.0 }
    }
}

impl<T: ?Sized> Pointer for UnsafeRef<T> {
    type Metadata = ();
    type Target = T;

    fn into_raw(this: Self) -> (*const Self::Target, Self::Metadata) {
        (this.as_ptr(), ())
    }
    unsafe fn from_raw(ptr: *const Self::Target, _: Self::Metadata) -> Self {
        Self::new(ptr)
    }
}

#[cfg(feature = "alloc")]
mod _alloc {
    use alloc::alloc::Allocator;
    use alloc::boxed::Box;
    use alloc::rc::Rc;
    use alloc::sync::Arc;

    use super::*;

    impl<T: ?Sized, A: Allocator> Pointer for Box<T, A> {
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

    impl<T: ?Sized> Pointer for Rc<T> {
        type Metadata = ();
        type Target = T;
        fn into_raw(this: Self) -> (*const Self::Target, Self::Metadata) {
            (Self::into_raw(this), ())
        }
        unsafe fn from_raw(ptr: *const Self::Target, _: Self::Metadata) -> Self {
            Self::from_raw(ptr)
        }
    }

    impl<T: ?Sized> Pointer for Arc<T> {
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
// #[cfg(feature = "alloc")]
// pub use _alloc::*;
