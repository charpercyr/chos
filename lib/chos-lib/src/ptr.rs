use core::{mem::align_of, ptr::write_bytes, sync::atomic::AtomicPtr};

pub unsafe fn write_bytes_slice<T>(ptr: *mut [T], value: u8) {
    write_bytes((*ptr).as_mut_ptr(), value, (*ptr).len())
}

pub const fn dangling_mut<T>() -> *mut T {
    align_of::<T>() as *mut T
}

pub const fn dangling<T>() -> *const T {
    dangling_mut()
}

pub const fn dangling_atomic<T>() -> AtomicPtr<T> {
    AtomicPtr::new(dangling_mut())
}
