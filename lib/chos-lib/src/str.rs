use core::slice::{from_raw_parts, from_raw_parts_mut};
use core::str::{
    from_utf8, from_utf8_mut, from_utf8_unchecked, from_utf8_unchecked_mut, Utf8Error,
};

pub enum BoundedError {
    Utf8Error(Utf8Error),
    BoundError,
}

pub unsafe fn strlen_bounded(s: *const u8, max: usize) -> Option<usize> {
    assert!(max <= isize::MAX as usize);
    let mut cur = s;
    while *cur != 0 {
        if (cur.offset_from(s) as usize) > max {
            return None;
        }
        cur = cur.offset(1);
    }
    Some(cur.offset_from(s) as usize)
}

pub unsafe fn strlen(s: *const u8) -> usize {
    strlen_bounded(s, isize::MAX as usize).unwrap()
}

pub unsafe fn from_cstring_utf8<'a>(s: *const u8) -> Result<&'a str, Utf8Error> {
    from_utf8(from_raw_parts(s, strlen(s)))
}

pub unsafe fn from_cstring_utf8_bounded<'a>(
    s: *const u8,
    max: usize,
) -> Result<&'a str, BoundedError> {
    let len = strlen_bounded(s, max).ok_or(BoundedError::BoundError)?;
    from_utf8(from_raw_parts(s, len)).map_err(BoundedError::Utf8Error)
}

pub unsafe fn from_cstring_utf8_mut<'a>(s: *mut u8) -> Result<&'a mut str, Utf8Error> {
    from_utf8_mut(from_raw_parts_mut(s, strlen(s)))
}

pub unsafe fn from_cstring_utf8_bounded_mut<'a>(
    s: *mut u8,
    max: usize,
) -> Result<&'a mut str, BoundedError> {
    let len = strlen_bounded(s, max).ok_or(BoundedError::BoundError)?;
    from_utf8_mut(from_raw_parts_mut(s, len)).map_err(BoundedError::Utf8Error)
}

pub unsafe fn from_cstring_utf8_unchecked<'a>(s: *const u8) -> &'a str {
    from_utf8_unchecked(from_raw_parts(s, strlen(s)))
}

pub unsafe fn from_cstring_utf8_bounded_unchecked<'a>(s: *const u8, max: usize) -> Option<&'a str> {
    let len = strlen_bounded(s, max)?;
    Some(from_utf8_unchecked(from_raw_parts(s, len)))
}

pub unsafe fn from_cstring_utf8_unchecked_mut<'a>(s: *mut u8) -> &'a mut str {
    from_utf8_unchecked_mut(from_raw_parts_mut(s, strlen(s)))
}

pub unsafe fn from_cstring_utf8_bounded_unchecked_mut<'a>(
    s: *mut u8,
    max: usize,
) -> Option<&'a mut str> {
    let len = strlen_bounded(s, max)?;
    Some(from_utf8_unchecked_mut(from_raw_parts_mut(s, len)))
}
