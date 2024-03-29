use core::marker::PhantomData;
use core::ops::Deref;
use core::slice;

use crate::int::CeilDiv;

pub mod raw;
mod util;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct InvalidSizeError;

pub enum InvalidTarEntry {
    InvalidSize,
}

pub struct Tar<'a> {
    bytes: &'a [u8],
}

impl<'a> Tar<'a> {
    pub fn new(bytes: &'a [u8]) -> Result<Self, InvalidSizeError> {
        (bytes.len() % 512 == 0)
            .then_some(Self { bytes })
            .ok_or(InvalidSizeError)
    }

    pub fn iter(&self) -> TarIter<'a> {
        TarIter {
            cur: self.bytes.as_ptr().cast(),
            end: unsafe { self.bytes.as_ptr().cast::<u8>().add(self.bytes.len()) },
            tar: PhantomData,
        }
    }
}

impl<'a> IntoIterator for &Tar<'a> {
    type Item = TarEntry<'a>;
    type IntoIter = TarIter<'a>;
    fn into_iter(self) -> Self::IntoIter {
        self.iter()
    }
}

pub struct TarEntry<'a> {
    header: &'a raw::FileHeader,
    contents: &'a [u8],
}

impl<'a> TarEntry<'a> {
    pub fn contents(&self) -> &'a [u8] {
        &self.contents
    }
}

impl Deref for TarEntry<'_> {
    type Target = raw::FileHeader;
    fn deref(&self) -> &Self::Target {
        self.header
    }
}

pub struct TarIter<'a> {
    cur: *const u8,
    end: *const u8,
    tar: PhantomData<&'a Tar<'a>>,
}
unsafe impl Send for TarIter<'_> {}
unsafe impl Sync for TarIter<'_> {}

impl<'a> TarIter<'a> {
    unsafe fn header(&self) -> &'a raw::FileHeader {
        &*self.cur.cast()
    }
}

impl<'a> Iterator for TarIter<'a> {
    type Item = TarEntry<'a>;
    fn next(&mut self) -> Option<Self::Item> {
        while self.cur < self.end {
            unsafe {
                let header = self.header();
                let size = header.size();
                if size != 0 || header.name().0.len() != 0 {
                    let contents_ptr = self.cur.add(512);
                    let contents = slice::from_raw_parts(contents_ptr, size as usize);
                    self.cur = contents_ptr.add(size.align_up(512) as usize);
                    return Some(TarEntry { contents, header });
                } else {
                    self.cur = self.cur.add(512);
                }
            }
        }
        None
    }
}
