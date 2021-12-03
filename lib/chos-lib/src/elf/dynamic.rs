use core::mem::size_of;
use core::slice::{from_raw_parts, Iter};

use super::raw::Elf64Dyn;
use super::{Elf, GnuHash, LookupStrategy, Rela, StrTab, Symtab};

#[derive(Clone, Copy)]
pub struct Dynamic<'a> {
    entries: &'a [Elf64Dyn],
}

impl<'a> Dynamic<'a> {
    pub(crate) unsafe fn new(buf: &'a [u8]) -> Self {
        Self {
            entries: from_raw_parts(buf.as_ptr().cast(), buf.len() / size_of::<Elf64Dyn>()),
        }
    }

    pub fn rela(&self, elf: &'a Elf<'a>) -> Option<Rela<'a>> {
        let mut rela = None;
        let mut relasz = None;
        for e in self {
            match e.typ() {
                DynamicEntryType::Rela => rela = Some(e.val()),
                DynamicEntryType::RelaSz => relasz = Some(e.val()),
                _ => (),
            }
            if let (Some(rela), Some(relasz)) = (rela, relasz) {
                unsafe { return Some(Rela::new(elf.get_buffer(rela as usize, relasz as usize))) }
            }
        }
        None
    }

    pub fn relaplt(&self, elf: &'a Elf<'a>) -> Option<Rela<'a>> {
        let mut rela = None;
        let mut relasz = None;
        let mut relaplt = false;
        for e in self {
            match e.typ() {
                DynamicEntryType::JmpRel => rela = Some(e.val()),
                DynamicEntryType::PltRelSz => relasz = Some(e.val()),
                DynamicEntryType::PltRel if e.val() == 7 => relaplt = true,
                _ => (),
            }
            if let (Some(rela), Some(relasz), true) = (rela, relasz, relaplt) {
                unsafe { return Some(Rela::new(elf.get_buffer(rela as usize, relasz as usize))) }
            }
        }
        None
    }

    pub fn strtab(&'a self, elf: &'a Elf<'a>) -> Option<StrTab<'a>> {
        let mut strtab = None;
        let mut strtabsz = None;
        for e in self {
            match e.typ() {
                DynamicEntryType::StrTab => strtab = Some(e.val()),
                DynamicEntryType::StrSz => strtabsz = Some(e.val()),
                _ => (),
            }
            if let (Some(strtab), Some(strtabsz)) = (strtab, strtabsz) {
                return Some(StrTab::new(
                    elf.get_buffer(strtab as usize, strtabsz as usize),
                ));
            }
        }
        None
    }

    pub fn symtab(&'a self, elf: &'a Elf<'a>) -> Option<Symtab<'a>> {
        self.iter()
            .find(|e| e.typ() == DynamicEntryType::SymTab)
            .map(|e| {
                for s in elf.sections() {
                    if s.offset() == e.val() {
                        let mut strat = LookupStrategy::Linear;
                        if let Some(gnu_hash) = self.gnu_hash(elf) {
                            strat = LookupStrategy::GnuHash(gnu_hash);
                        }
                        return unsafe {
                            Some(Symtab::new(
                                elf.get_buffer(s.offset() as usize, s.size() as usize),
                                strat,
                            ))
                        };
                    }
                }
                None
            })
            .flatten()
    }

    pub fn gnu_hash(&'a self, elf: &'a Elf<'a>) -> Option<GnuHash<'a>> {
        self.iter()
            .find(|e| e.typ() == DynamicEntryType::GnuHash)
            .map(|e| {
                for s in elf.sections() {
                    if s.offset() == e.val() {
                        return unsafe {
                            Some(GnuHash::new(
                                elf.get_buffer(s.offset() as usize, s.size() as usize),
                            ))
                        };
                    }
                }
                None
            })
            .flatten()
    }
}
crate::elf_table!('a, Dynamic, entries, DynamicEntry, DynamicEntryIter, Iter<'a, Elf64Dyn>);

#[derive(Clone, Copy)]
pub struct DynamicEntry<'a> {
    hdr: &'a Elf64Dyn,
}

impl<'a> DynamicEntry<'a> {
    fn new(hdr: &'a Elf64Dyn) -> Self {
        Self { hdr }
    }

    pub fn typ(&self) -> DynamicEntryType {
        use DynamicEntryType::*;
        match self.hdr.tag {
            0 => Null,
            2 => PltRelSz,
            3 => PltGot,
            5 => StrTab,
            6 => SymTab,
            7 => Rela,
            8 => RelaSz,
            9 => RelaEnt,
            10 => StrSz,
            11 => SymEnt,
            12 => Init,
            20 => PltRel,
            23 => JmpRel,
            25 => InitArray,
            27 => InitArraySz,
            0x6ffffef5 => GnuHash,
            t => Unknown(t),
        }
    }

    pub fn val(&self) -> u64 {
        self.hdr.val
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DynamicEntryType {
    Null,
    PltRelSz,
    PltGot,
    StrTab,
    SymTab,
    Rela,
    RelaSz,
    RelaEnt,
    StrSz,
    SymEnt,
    Init,
    PltRel,
    JmpRel,
    InitArray,
    InitArraySz,
    GnuHash,
    Unknown(u64),
}
