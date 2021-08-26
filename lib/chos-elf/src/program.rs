use bitflags::bitflags;
use chos_lib::stride::{from_raw_parts, StrideSlice, StrideSliceIter};

use crate::raw::Elf64Phdr;
use crate::{Dynamic, Elf};

#[derive(Clone, Copy)]
pub struct Program<'a> {
    entries: StrideSlice<'a, Elf64Phdr>,
}

impl<'a> Program<'a> {
    pub(crate) unsafe fn new(buf: &'a [u8], entsize: usize) -> Self {
        Self {
            entries: from_raw_parts(buf.as_ptr().cast(), buf.len() / entsize, entsize),
        }
    }

    pub fn dynamic(&'a self, elf: &'a Elf<'a>) -> Option<Dynamic<'a>> {
        self.iter()
            .find(|p| p.typ() == ProgramEntryType::Dynamic)
            .map(|d| d.as_dynamic(elf))
            .flatten()
    }
}
crate::elf_table!('a, Program, entries, ProgramEntry, ProgramEntryIter, StrideSliceIter<'a, Elf64Phdr>);

#[derive(Clone, Copy)]
pub struct ProgramEntry<'a> {
    hdr: &'a Elf64Phdr,
}

impl<'a> ProgramEntry<'a> {
    fn new(hdr: &'a Elf64Phdr) -> Self {
        Self { hdr }
    }

    pub fn typ(&self) -> ProgramEntryType {
        use ProgramEntryType::*;
        match self.hdr.typ {
            0 => Null,
            1 => Load,
            2 => Dynamic,
            6 => Phdr,
            t => Unknown(t),
        }
    }

    pub fn flags(&self) -> ProgramEntryFlags {
        ProgramEntryFlags::from_bits_truncate(self.hdr.flags)
    }

    pub fn offset(&self) -> u64 {
        self.hdr.off
    }

    pub fn align(&self) -> u64 {
        self.hdr.align
    }

    pub fn vaddr(&self) -> u64 {
        self.hdr.vaddr
    }

    pub fn paddr(&self) -> u64 {
        self.hdr.paddr
    }

    pub fn file_size(&self) -> u64 {
        self.hdr.filesz
    }

    pub fn mem_size(&self) -> u64 {
        self.hdr.memsz
    }

    pub fn as_dynamic(&self, elf: &Elf<'a>) -> Option<Dynamic<'a>> {
        if self.typ() == ProgramEntryType::Dynamic {
            unsafe {
                Some(Dynamic::new(elf.get_buffer(
                    self.offset() as usize,
                    self.file_size() as usize,
                )))
            }
        } else {
            None
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ProgramEntryType {
    Null,
    Load,
    Dynamic,
    Phdr,
    Unknown(u32),
}

bitflags! {
    pub struct ProgramEntryFlags: u32 {
        const EXEC = 0b001;
        const WRITE = 0b010;
        const READ = 0b100;
    }
}
