#![no_std]

pub mod raw;

mod section;
use raw::Elf64Shdr;
pub use section::*;

mod strtab;
pub use strtab::*;

mod symtab;
pub use symtab::*;

use core::marker::PhantomData;

pub enum ElfError {
    InvalidSignature,
}

pub struct Elf64<'a> {
    hdr: *const raw::Elf64Hdr,
    _ref: PhantomData<&'a [u8]>,
}

impl<'a> Elf64<'a> {
    pub fn from_bytes(bytes: &'a [u8]) -> Result<Self, ElfError> {
        unsafe {
            let hdr: *const raw::Elf64Hdr = bytes.as_ptr().cast();
            let hdr = &*hdr;
            if hdr.ident.magic == raw::MAGIC && hdr.ident.class == raw::CLASS64 {
                Ok(Self {
                    hdr,
                    _ref: PhantomData,
                })
            } else {
                Err(ElfError::InvalidSignature)
            }
        }
    }
    pub unsafe fn from_bytes_unchecked(bytes: &'a [u8]) -> Self {
        Self {
            hdr: bytes.as_ptr().cast(),
            _ref: PhantomData,
        }
    }

    pub fn sections(&self) -> SectionTable<'a> {
        let hdr = self.raw();
        let shstrt = self.hdr as usize + hdr.shoff as usize + (hdr.shstrndx as usize) * (hdr.shentsize as usize);
        let shstrt = shstrt as *const Elf64Shdr;
        unsafe {
            let shstrt = &*(shstrt);
            SectionTable::new(
                (self.hdr as *const u8).add(hdr.shoff as usize),
                hdr.shentsize as _,
                hdr.shnum as _,
                StringTable::new((self.hdr as *const u8).offset(shstrt.off as _), shstrt.size as _),
            )
        }
    }

    pub fn raw(&self) -> &'a raw::Elf64Hdr {
        unsafe { &*self.hdr }
    }
}
