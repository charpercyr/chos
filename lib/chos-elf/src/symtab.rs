use core::marker::PhantomData;
use core::mem::size_of;

use chos_lib::stride::{self, StrideSlice};

use crate::raw::Elf64Sym;
use crate::StringTable;

#[derive(Clone, Copy, Debug)]
pub struct SymbolTable<'a> {
    entries: StrideSlice<'a, Elf64Sym>,
    strings: StringTable<'a>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SymbolBinding {
    Local,
    Global,
    Weak,
    Other(u8),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SymbolType {
    NoType,
    Object,
    Func,
    Section,
    File,
    Common,
    Tls,
    Other(u8),
}

impl<'a> SymbolTable<'a> {
    pub unsafe fn new(entries: *const u8, size: usize, strings: StringTable<'a>) -> Self {
        Self::new_with_stride(entries, size, size_of::<Elf64Sym>(), strings)
    }
    pub unsafe fn new_with_stride(
        entries: *const u8,
        size: usize,
        entsize: usize,
        strings: StringTable<'a>,
    ) -> Self {
        assert!(size % entsize == 0);
        Self {
            entries: stride::from_raw_parts(entries as *const _, size / entsize, entsize),
            strings,
        }
    }

    pub fn iter(&'a self) -> SymbolTableIter<'a> {
        SymbolTableIter {
            iter: self.entries.iter(),
            symtab: self,
        }
    }
}

impl<'a> IntoIterator for &'a SymbolTable<'a> {
    type Item = Symbol<'a>;
    type IntoIter = SymbolTableIter<'a>;
    fn into_iter(self) -> Self::IntoIter {
        self.iter()
    }
}

pub struct SymbolTableIter<'a> {
    iter: stride::StrideSliceIter<'a, Elf64Sym>,
    symtab: &'a SymbolTable<'a>,
}

impl<'a> Iterator for SymbolTableIter<'a> {
    type Item = Symbol<'a>;

    fn next(&mut self) -> Option<Symbol<'a>> {
        self.iter.next().map(|entry| Symbol {
            entry,
            symtab: self.symtab,
        })
    }
}

pub struct Symbol<'a> {
    entry: &'a Elf64Sym,
    symtab: &'a SymbolTable<'a>,
}

impl<'a> Symbol<'a> {
    pub fn name(&self) -> &'a str {
        self.symtab.strings.get_string(self.raw().name as usize)
    }

    pub fn binding(&self) -> SymbolBinding {
        match (self.raw().info & 0xf0) >> 4 {
            0 => SymbolBinding::Local,
            1 => SymbolBinding::Global,
            2 => SymbolBinding::Weak,
            b => SymbolBinding::Other(b),
        }
    }

    pub fn typ(&self) -> SymbolType {
        match self.raw().info & 0xf {
            0 => SymbolType::NoType,
            1 => SymbolType::Object,
            2 => SymbolType::Func,
            3 => SymbolType::Section,
            4 => SymbolType::File,
            5 => SymbolType::Common,
            6 => SymbolType::Tls,
            t => SymbolType::Other(t),
        }
    }

    pub fn shndx(&self) -> u16 {
        self.raw().shndx
    }

    pub fn addr(&self) -> u64 {
        self.raw().addr
    }

    pub fn size(&self) -> u64 {
        self.raw().size
    }

    pub fn raw(&self) -> &'a Elf64Sym {
        self.entry
    }
}
