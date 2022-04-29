#[cfg(feature = "alloc")]
use alloc::borrow::Cow;
use core::marker::PhantomData;
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

pub struct TarEntry<'a> {
    header: &'a raw::FileHeader,
    contents: &'a [u8],
}

impl<'a> TarEntry<'a> {
    pub fn contents(&self) -> &'a [u8] {
        &self.contents
    }

    pub fn name(&self) -> (&str, Option<&str>) {
        self.header.name()
    }

    #[cfg(feature = "alloc")]
    pub fn name_merged(&self) -> Cow<'_, str> {
        self.header.name_merged()
    }
}

pub struct TarIter<'a> {
    cur: *const u8,
    end: *const u8,
    tar: PhantomData<&'a Tar<'a>>,
}

impl<'a> TarIter<'a> {
    unsafe fn header(&self) -> &'a raw::FileHeader {
        &*self.cur.cast()
    }
}

impl<'a> Iterator for TarIter<'a> {
    type Item = TarEntry<'a>;
    fn next(&mut self) -> Option<Self::Item> {
        if self.cur < self.end {
            unsafe {
                let header = self.header();
                let size = header.size();
                if size == 0 && header.name().0.len() == 0 {
                    return None;
                }
                let contents_ptr = self.cur.add(512);
                let contents = slice::from_raw_parts(contents_ptr, size as usize);
                self.cur = contents_ptr.add(size.align_up(512) as usize);
                Some(TarEntry { contents, header })
            }
        } else {
            None
        }
    }
}
