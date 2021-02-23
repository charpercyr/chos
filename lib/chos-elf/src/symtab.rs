
use core::marker::PhantomData;
use core::mem::size_of;

use crate::StringTable;
use crate::raw::Elf64Sym;

pub struct SymbolTable<'a> {
    entries: *const Elf64Sym,
    size: usize,
    strings: &'a StringTable<'a>,
    _ref: PhantomData<&'a [Elf64Sym]>,
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
    Other(u8),
}

impl<'a> SymbolTable<'a> {
    pub unsafe fn new(entries: *const u8, size: usize, strings: &'a StringTable<'a>) -> Self {
        assert!(size % size_of::<Elf64Sym>() == 0);
        Self {
            entries: entries.cast(),
            size: size / size_of::<Elf64Sym>(),
            strings,
            _ref: PhantomData,
        }
    }

    pub fn symbols(&self) -> Symbols<'_> {
        Symbols {
            cur: self.entries,
            symtab: self,
        }
    }
}

pub struct Symbols<'a> {
    cur: *const Elf64Sym,
    symtab: &'a SymbolTable<'a>,
}

impl<'a> Iterator for Symbols<'a> {
    type Item = Symbol<'a>;

    fn next(&mut self) -> Option<Symbol<'a>> {
        unsafe {
            if (self.cur.offset_from(self.symtab.entries) as usize) <= self.symtab.size {
                let sym = Symbol {
                    entry: self.cur,
                    symtab: self.symtab,
                };
                self.cur = self.cur.offset(1);
                Some(sym)
            } else {
                None
            }
        }
    }
}

pub struct Symbol<'a> {
    entry: *const Elf64Sym,
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

    pub fn raw(&self) -> &Elf64Sym {
        unsafe { &*self.entry }
    }
}
