
use core::marker::PhantomData;

use chos_lib::str::from_cstring_utf8_bounded_unchecked;

pub struct StringTable<'a> {
    ptr: *const u8,
    size: usize,
    _ref: PhantomData<&'a [u8]>,
}

impl StringTable<'_> {
    pub unsafe fn new(ptr: *const u8, size: usize) -> Self {
        Self {
            ptr,
            size,
            _ref: PhantomData,
        }
    }

    pub fn try_get_string(&self, idx: usize) -> Option<&str> {
        if idx >= self.size {
            return None;
        }
        unsafe { from_cstring_utf8_bounded_unchecked(
            self.ptr.offset(idx as isize),
            self.size - idx,
        ) }
    }

    pub fn get_string(&self, idx: usize) -> &str {
        self.try_get_string(idx).unwrap()
    }
}
