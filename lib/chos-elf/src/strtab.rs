use core::marker::PhantomData;

use chos_lib::str::from_cstring_utf8_bounded_unchecked;

#[derive(Clone, Copy, Debug)]
pub struct StringTable<'a> {
    ptr: *const u8,
    size: usize,
    _ref: PhantomData<&'a [u8]>,
}

impl<'a> StringTable<'a> {
    pub unsafe fn new(ptr: *const u8, size: usize) -> Self {
        Self {
            ptr,
            size,
            _ref: PhantomData,
        }
    }

    pub fn try_get_string(&self, idx: usize) -> Option<&'a str> {
        if idx >= self.size {
            return None;
        }
        unsafe {
            from_cstring_utf8_bounded_unchecked(self.ptr.offset(idx as isize), self.size - idx)
        }
    }

    pub fn get_string(&self, idx: usize) -> &'a str {
        assert!(idx < self.size, "Index [{}] out of bounds", idx);
        self.try_get_string(idx).unwrap()
    }
}
