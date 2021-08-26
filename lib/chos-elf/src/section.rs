use core::mem::transmute;

use bitflags::bitflags;
use chos_lib::stride::{from_raw_parts, StrideSlice, StrideSliceIter};

use crate::raw::Elf64Shdr;
use crate::{Elf, LookupStrategy, StrTab, Symtab};

pub struct Sections<'a> {
    entries: StrideSlice<'a, Elf64Shdr>,
}
impl<'a> Sections<'a> {
    pub(crate) unsafe fn new(buf: &'a [u8], entsize: usize) -> Self {
        Self {
            entries: from_raw_parts(buf.as_ptr().cast(), buf.len() / entsize, entsize),
        }
    }
}
crate::elf_table!('a, Sections, entries, SectionEntry, SectionEntryIter, StrideSliceIter<'a, Elf64Shdr>);

pub struct SectionEntry<'a> {
    hdr: &'a Elf64Shdr,
}
impl<'a> SectionEntry<'a> {
    fn new(hdr: &'a Elf64Shdr) -> Self {
        Self { hdr }
    }

    pub fn name(&'a self, elf: &'a Elf<'a>) -> Option<&'a str> {
        let sections = elf.sections();
        let sec = sections.get(elf.raw().shstrndx as usize);
        unsafe { transmute(sec.as_strtab(elf)?.get_string(self.hdr.name as usize)) }
    }

    pub fn typ(&self) -> SectionEntryType {
        use SectionEntryType::*;
        match self.hdr.typ {
            0 => Null,
            1 => Progbits,
            2 => Symtab,
            3 => Strtab,
            4 => Rela,
            6 => Dynamic,
            8 => Nobits,
            11 => DynSym,
            14 => InitArray,
            0x6ffffff6 => GnuHash,
            t => Unknown(t),
        }
    }

    pub fn flags(&self) -> SectionEntryFlags {
        SectionEntryFlags::from_bits_truncate(self.hdr.flags)
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

    pub fn as_symtab(&'a self, elf: &Elf<'a>) -> Option<Symtab<'a>> {
        if self.typ() == SectionEntryType::Symtab || self.typ() == SectionEntryType::DynSym {
            unsafe {
                Some(Symtab::new(
                    elf.get_buffer(self.hdr.off as usize, self.hdr.size as usize),
                    LookupStrategy::Linear,
                ))
            }
        } else {
            None
        }
    }

    pub fn as_strtab(&'a self, elf: &'a Elf<'a>) -> Option<StrTab<'a>> {
        if self.typ() == SectionEntryType::Strtab {
            Some(StrTab::new(
                elf.get_buffer(self.hdr.off as usize, self.hdr.size as usize),
            ))
        } else {
            None
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SectionEntryType {
    Null,
    Progbits,
    Symtab,
    Strtab,
    Rela,
    Dynamic,
    Nobits,
    DynSym,
    InitArray,
    GnuHash,
    Unknown(u32),
}

bitflags! {
    pub struct SectionEntryFlags: u64 {
        const WRITE = 0x1;
        const ALLOC = 0x2;
        const EXECINSTR = 0x4;
        const STRINGS = 0x20;
        const TLS = 0x400;
    }
}
