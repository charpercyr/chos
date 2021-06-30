
use crate::raw::Elf64Rela;

use core::mem::size_of;
use core::slice::{from_raw_parts, Iter};

#[derive(Clone, Copy)]
pub struct Rela<'a> {
    entries: &'a [Elf64Rela],
}

impl<'a> Rela<'a> {
    pub(crate) unsafe fn new(buf: &'a [u8]) -> Self {
        Self {
            entries: from_raw_parts(buf.as_ptr().cast(), buf.len() / size_of::<Elf64Rela>()),
        }
    }
}
crate::elf_table!('a, Rela, entries, RelaEntry, RelaEntryIter, Iter<'a, Elf64Rela>);

#[derive(Clone, Copy)]
pub struct RelaEntry<'a> {
    hdr: &'a Elf64Rela,
}
impl<'a> RelaEntry<'a> {
    fn new(hdr: &'a Elf64Rela) -> Self {
        Self { hdr }
    }

    pub fn offset(&self) -> u64 {
        self.hdr.off
    }

    pub fn info(&self) -> u64 {
        self.hdr.info
    }

    pub fn addend(&self) -> i64 {
        self.hdr.addend
    }

    pub fn sym(&self) -> u32 {
        (self.hdr.info >> 32) as u32
    }

    pub fn typ(&self) -> u32 {
        self.hdr.info as u32
    }

    pub fn x64_typ(&self) -> X64RelaType {
        use X64RelaType::*;
        match self.typ() {
            0 => None,
            1 => _64,
            2 => Pc32,
            3 => Got32,
            4 => Plt32,
            5 => Copy,
            6 => GlobDat,
            7 => JumpSlot,
            8 => Relative,
            9 => GotPcRel,
            10 => _32,
            11 => _32S,
            12 => _16,
            13 => Pc16,
            14 => _8,
            15 => Pc8,
            16 => DptMod64,
            17 => DtpOff64,
            18 => TpOff64,
            19 => TlsGd,
            20 => TlsLd,
            21 => DtpOff32,
            22 => GotTpOff,
            23 => TpOff32,
            24 => Pc64,
            25 => GotOff64,
            26 => GotPc32,
            t => Unknown(t),
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum X64RelaType {
    None,
    _64,
    Pc32,
    Got32,
    Plt32,
    Copy,
    GlobDat,
    JumpSlot,
    Relative,
    GotPcRel,
    _32,
    _32S,
    _16,
    Pc16,
    _8,
    Pc8,
    DptMod64,
    DtpOff64,
    TpOff64,
    TlsGd,
    TlsLd,
    DtpOff32,
    GotTpOff,
    TpOff32,
    Pc64,
    GotOff64,
    GotPc32,
    Unknown(u32),
}
