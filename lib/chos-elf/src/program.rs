use core::fmt;

use bitflags::bitflags;
use chos_lib::stride::{self, StrideSlice, StrideSliceIter};

use super::raw::Elf64Phdr;

#[derive(Clone, Copy, Debug)]
pub struct ProgramTable<'a> {
    entries: StrideSlice<'a, Elf64Phdr>,
}

impl<'a> ProgramTable<'a> {
    pub unsafe fn new(ptr: *const u8, entsize: usize, len: usize) -> Self {
        Self {
            entries: stride::from_raw_parts(ptr.cast(), len, entsize),
        }
    }

    pub fn get(&'a self, idx: usize) -> ProgramEntry<'a> {
        ProgramEntry {
            hdr: &self.entries[idx],
        }
    }

    pub fn iter(&self) -> ProgramTableIter<'a> {
        ProgramTableIter {
            iter: self.entries.iter(),
        }
    }
}

impl<'a> IntoIterator for ProgramTable<'a> {
    type Item = ProgramEntry<'a>;
    type IntoIter = ProgramTableIter<'a>;
    fn into_iter(self) -> Self::IntoIter {
        self.iter()
    }
}

impl<'a> IntoIterator for &ProgramTable<'a> {
    type Item = ProgramEntry<'a>;
    type IntoIter = ProgramTableIter<'a>;
    fn into_iter(self) -> Self::IntoIter {
        self.iter()
    }
}

#[derive(Clone, Copy)]
pub struct ProgramTableIter<'a> {
    iter: StrideSliceIter<'a, Elf64Phdr>,
}

impl<'a> Iterator for ProgramTableIter<'a> {
    type Item = ProgramEntry<'a>;
    fn next(&mut self) -> Option<Self::Item> {
        self.iter.next().map(|hdr| ProgramEntry { hdr })
    }
}

#[derive(Clone, Copy)]
pub struct ProgramEntry<'a> {
    hdr: &'a Elf64Phdr,
}

impl ProgramEntry<'_> {
    pub fn typ(&self) -> ProgramEntryType {
        use ProgramEntryType::*;
        match self.hdr.typ {
            0 => Null,
            1 => Load,
            2 => Dynamic,
            3 => Interp,
            4 => Note,
            5 => Shlib,
            6 => Phdr,
            7 => Tls,
            t @ 0x6000_0000..=0x6fff_ffff => Os(t),
            t @ 0x7000_0000..=0x7fff_ffff => Proc(t),
            t => Unknown(t),
        }
    }

    pub fn flags(&self) -> ProgramEntryFlags {
        ProgramEntryFlags::from_bits_truncate(self.hdr.flags)
    }

    pub fn offset(&self) -> u64 {
        self.hdr.off
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

    pub fn align(&self) -> u64 {
        self.hdr.align
    }
}

impl fmt::Debug for ProgramEntry<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("ProgramEntry")
            .field("typ", &self.typ())
            .field("flags", &self.flags())
            .field("offset", &self.vaddr())
            .field("vaddr", &self.vaddr())
            .field("paddr", &self.paddr())
            .field("file_size", &self.file_size())
            .field("mem_size", &self.mem_size())
            .field("align", &self.align())
            .finish()
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum ProgramEntryType {
    Null,
    Load,
    Dynamic,
    Interp,
    Note,
    Shlib,
    Phdr,
    Tls,
    Os(u32),
    Proc(u32),
    Unknown(u32),
}

bitflags! {
    pub struct ProgramEntryFlags: u32 {
        const EXECUTE = 0x1;
        const WRITE = 0x2;
        const READ = 0x4;
    }
}