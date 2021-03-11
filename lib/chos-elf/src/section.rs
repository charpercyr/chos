use crate::{raw::Elf64Shdr, StringTable, SymbolTable};

use bitflags::bitflags;

use core::marker::PhantomData;

use chos_lib::stride;

#[derive(Clone, Copy)]
pub struct SectionTable<'a> {
    base: *const u8,
    entries: stride::StrideSlice<'a, Elf64Shdr>,
    string_table: StringTable<'a>,
    _ref: PhantomData<&'a [u8]>,
}

impl<'a> SectionTable<'a> {
    pub unsafe fn new(
        base: *const u8,
        ptr: *const u8,
        entsize: usize,
        len: usize,
        string_table: StringTable<'a>,
    ) -> Self {
        Self {
            base,
            entries: stride::from_raw_parts(ptr.cast(), len, entsize),
            string_table,
            _ref: PhantomData,
        }
    }

    pub fn get(&'a self, idx: usize) -> Section<'a> {
        Section {
            hdr: &self.entries[idx],
            table: self,
        }
    }

    pub fn iter(&'a self) -> SectionTableIter<'a> {
        SectionTableIter {
            iter: self.entries.iter(),
            table: self,
        }
    }
}

impl<'a> IntoIterator for &'a SectionTable<'a> {
    type Item = Section<'a>;
    type IntoIter = SectionTableIter<'a>;
    fn into_iter(self) -> Self::IntoIter {
        self.iter()
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
    pub fn name(&self) -> &'a str {
        self.table.string_table.get_string(self.hdr.name as _)
    }

    pub fn typ(&self) -> SectionType {
        use SectionType::*;
        match self.hdr.typ {
            0 => Null,
            1 => Progbits,
            2 => Symtab,
            3 => Strtab,
            4 => Rela,
            5 => Hash,
            6 => Dynamic,
            7 => Note,
            8 => Nobits,
            9 => Rel,
            10 => Shlib,
            11 => Dynsym,
            t @ 0x6000_0000..=0x6fff_ffff => Os(t),
            t @ 0x7000_0000..=0x7fff_ffff => Proc(t),
            t @ 0x8000_0000..=0x8fff_ffff => User(t),
            t => Unknown(t),
        }
    }

    pub fn flags(&self) -> SectionFlags {
        SectionFlags::from_bits_truncate(self.hdr.flags)
    }

    pub fn addr(&self) -> u64 {
        self.hdr.addr
    }

    pub fn offset(&self) -> u64 {
        self.hdr.off
    }

    pub fn size(&self) -> u64 {
        self.hdr.size
    }

    pub fn link(&self) -> u32 {
        self.hdr.link
    }

    pub fn info(&self) -> u32 {
        self.hdr.info
    }

    pub fn addr_align(&self) -> u64 {
        self.hdr.addr_align
    }

    pub fn entsize(&self) -> u64 {
        self.hdr.entsize
    }

    pub fn bytes(&self) -> &'a [u8] {
        unsafe {
            let data = self.table.base.add(self.hdr.off as usize);
            let len = self.hdr.size as usize;
            core::slice::from_raw_parts(data, len)
        }
    }

    pub fn as_strtab(&self) -> Option<StringTable<'a>> {
        (self.typ() == SectionType::Strtab).then(|| {
            let data = self.bytes();
            unsafe { StringTable::new(data.as_ptr(), data.len()) }
        })
    }

    pub fn as_symtab(&self) -> Option<SymbolTable<'a>> {
        let typ = self.typ();
        (typ == SectionType::Symtab || typ == SectionType::Dynsym).then(|| {
            let data = self.bytes();
            let strtab = self.hdr.link as usize;
            unsafe {
                SymbolTable::new_with_stride(
                    data.as_ptr(),
                    data.len(),
                    self.entsize() as usize,
                    self.table.get(strtab).as_strtab().unwrap(),
                )
            }
        })
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum SectionType {
    Null,
    Progbits,
    Symtab,
    Strtab,
    Rela,
    Hash,
    Dynamic,
    Note,
    Nobits,
    Rel,
    Shlib,
    Dynsym,
    Os(u32),
    Proc(u32),
    User(u32),
    Unknown(u32),
}

bitflags! {
    pub struct SectionFlags: u64 {
        const WRITE = 0x1;
        const ALLOC = 0x2;
        const EXECINSTR = 0x4;
        const MERGE = 0x10;
        const STRINGS = 0x20;
        const INFO_LINK = 0x40;
        const LINK_ORDER = 0x80;
        const OS_NONCONFORMING = 0x100;
        const GROUP = 0x200;
        const TLS  = 0x400;
    }
}
