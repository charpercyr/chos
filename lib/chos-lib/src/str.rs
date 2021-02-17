use core::slice::{from_raw_parts, from_raw_parts_mut};
use core::str::{
    from_utf8,
    from_utf8_mut,
    from_utf8_unchecked,
    from_utf8_unchecked_mut,
    Utf8Error,
};

pub unsafe fn strlen(s: *const u8) -> usize {
    let mut cur = s;
    while *cur != 0 {
        cur = cur.offset(1)
    }
    cur.offset_from(s) as usize
}

pub unsafe fn from_cstring_utf8<'a>(s: *const u8) -> Result<&'a str, Utf8Error> {
    from_utf8(from_raw_parts(s, strlen(s)))
}

pub unsafe fn from_cstring_utf8_mut<'a>(s: *mut u8) -> Result<&'a mut str, Utf8Error> {
    from_utf8_mut(from_raw_parts_mut(s, strlen(s)))
}

pub unsafe fn from_cstring_utf8_unchecked<'a>(s: *const u8) -> &'a str {
    from_utf8_unchecked(from_raw_parts(s, strlen(s)))
}

pub unsafe fn from_cstring_utf8_unchecked_mut<'a>(s: *mut u8) -> &'a mut str {
    from_utf8_unchecked_mut(from_raw_parts_mut(s, strlen(s)))
}