
pub mod slist;

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

pub trait Pointer {
    type Target: ?Sized;

    fn into_raw(this: Self) -> *const Self::Target;
    unsafe fn from_raw(ptr: *const Self::Target) -> Self;
}

impl<T: ?Sized> Pointer for &T {
    type Target = T;

    fn into_raw(this: Self) -> *const Self::Target {
        this
    }
    unsafe fn from_raw(ptr: *const Self::Target) -> Self {
        &*ptr
    }
}

impl<T: ?Sized> Pointer for &mut T {
    type Target = T;

    fn into_raw(this: Self) -> *const Self::Target {
        this
    }
    unsafe fn from_raw(ptr: *const Self::Target) -> Self {
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
    type Target = T;
    
    fn into_raw(this: Self) -> *const Self::Target {
        this.as_ptr()
    }
    unsafe fn from_raw(ptr: *const Self::Target) -> Self {
        Self::new(ptr)
    }
}