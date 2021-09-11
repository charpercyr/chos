use core::mem::size_of;
use core::slice::{from_raw_parts, Iter};

use super::raw::Elf64Sym;
use super::{GnuHash, StrTab};

#[derive(Copy, Clone)]
pub enum LookupStrategy<'a> {
    Linear,
    GnuHash(GnuHash<'a>),
}

#[derive(Copy, Clone)]
pub struct Symtab<'a> {
    entries: &'a [Elf64Sym],
    strat: LookupStrategy<'a>,
}

impl<'a> Symtab<'a> {
    pub unsafe fn new(buf: &'a [u8], strat: LookupStrategy<'a>) -> Self {
        Self {
            entries: from_raw_parts(buf.as_ptr().cast(), buf.len() / size_of::<Elf64Sym>()),
            strat,
        }
    }

    pub fn lookup(&'a self, name: &str, strtab: &'a StrTab<'a>) -> Option<SymtabEntry<'a>> {
        match self.strat {
            LookupStrategy::Linear => self.lookup_linear(name, strtab),
            LookupStrategy::GnuHash(h) => h.lookup(name, strtab, self),
        }
    }

    fn lookup_linear(&self, name: &str, strtab: &StrTab<'a>) -> Option<SymtabEntry<'a>> {
        for sym in self {
            if let Some(symname) = sym.name(strtab) {
                if symname == name {
                    return Some(sym);
                }
            }
        }
        None
    }
}
crate::elf_table!('a, Symtab, entries, SymtabEntry, SymtabEntryIter, Iter<'a, Elf64Sym>);

#[derive(Copy, Clone)]
pub struct SymtabEntry<'a> {
    hdr: &'a Elf64Sym,
}
impl<'a> SymtabEntry<'a> {
    fn new(hdr: &'a Elf64Sym) -> Self {
        Self { hdr }
    }

    pub fn name(&self, strtab: &'a StrTab<'a>) -> Option<&'a str> {
        strtab.get_string(self.hdr.name as usize)
    }

    pub fn info(&self) -> u8 {
        self.hdr.info
    }

    pub fn bind(&self) -> SymtabEntryBind {
        use SymtabEntryBind::*;
        match self.hdr.info >> 4 {
            0 => Local,
            1 => Global,
            2 => Weak,
            b => SymtabEntryBind::Unknown(b),
        }
    }

    pub fn typ(&self) -> SymtabEntryType {
        use SymtabEntryType::*;
        match self.hdr.info & 0xf {
            0 => NoType,
            1 => Object,
            2 => Func,
            3 => Section,
            4 => File,
            5 => Common,
            6 => Tls,
            t => Unknown(t),
        }
    }

    pub fn other(&self) -> u8 {
        self.hdr.other
    }

    pub fn shndx(&self) -> u16 {
        self.hdr.shndx
    }

    pub fn value(&self) -> u64 {
        self.hdr.value
    }

    pub fn size(&self) -> u64 {
        self.hdr.size
    }

    pub fn raw(&self) -> &Elf64Sym {
        &self.hdr
    }
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum SymtabEntryBind {
    Local,
    Global,
    Weak,
    Unknown(u8),
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum SymtabEntryType {
    NoType,
    Object,
    Func,
    Section,
    File,
    Common,
    Tls,
    Unknown(u8),
}
