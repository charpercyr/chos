
use core::ptr::write_bytes;

pub unsafe fn write_bytes_slice<T>(ptr: *mut [T], value: u8) {
    write_bytes((*ptr).as_mut_ptr(), value, (*ptr).len())
}
