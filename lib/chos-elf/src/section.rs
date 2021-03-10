
use core::marker::PhantomData;

use crate::{StringTable, raw::Elf64Shdr};

use chos_lib::stride;

#[derive(Clone, Copy)]
pub struct SectionTable<'a> {
    entries: stride::StrideSlice<'a, Elf64Shdr>,
    string_table: StringTable<'a>,
    _ref: PhantomData<&'a [u8]>,
}

impl<'a> SectionTable<'a> {
    pub unsafe fn new(ptr: *const u8, entsize: usize, entlen: usize, string_table: StringTable<'a>) -> Self {
        Self {
            entries: stride::from_raw_parts(ptr.cast(), entlen, entsize),
            string_table,
            _ref: PhantomData,
        }
    }

    pub fn sections(&'a self) -> SectionTableIter<'a> {
        SectionTableIter {
            iter: self.entries.iter(),
            table: self,
        }
    }
}

pub struct SectionTableIter<'a> {
    iter: stride::StrideSliceIter<'a, Elf64Shdr>,
    table: &'a SectionTable<'a>,
}

impl<'a> Iterator for SectionTableIter<'a> {
    type Item = Section<'a>;
    fn next(&mut self) -> Option<Self::Item> {
        self.iter.next().map(|hdr| Section { 
            hdr,
            table: self.table,
        })
    }
}

pub struct Section<'a> {
    hdr: &'a Elf64Shdr,
    table: &'a SectionTable<'a>,
}

impl<'a> Section<'a> {
    pub fn name(&self) -> &str {
        self.table.string_table.get_string(self.hdr.name as _)
    }
}