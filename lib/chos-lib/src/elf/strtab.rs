use crate::str::from_cstring_utf8_bounded;

#[derive(Clone, Copy)]
pub struct StrTab<'a> {
    data: &'a [u8],
}

impl<'a> StrTab<'a> {
    pub fn new(data: &'a [u8]) -> Self {
        Self { data }
    }

    pub fn get_string(&'a self, offset: usize) -> Option<&'a str> {
        if offset >= self.data.len() {
            return None;
        }
        unsafe {
            from_cstring_utf8_bounded(self.data.as_ptr().add(offset), self.data.len() - offset).ok()
        }
    }
}
