#![no_std]

pub mod raw;

mod program;
pub use program::*;

mod section;
pub use section::*;

mod strtab;
pub use strtab::*;

mod symtab;
pub use symtab::*;

use core::marker::PhantomData;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ElfError {
    InvalidSignature,
    InvalidBuffer,
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
            let file_sz = usize::max(
                (hdr.shoff as usize) + (hdr.shentsize as usize) * (hdr.shnum as usize),
                (hdr.phoff as usize) + (hdr.phentsize as usize) * (hdr.phnum as usize),
            );
            if hdr.ident.magic != raw::MAGIC || hdr.ident.class != raw::CLASS64 {
                Err(ElfError::InvalidSignature)
            } else if file_sz > bytes.len() {
                Err(ElfError::InvalidBuffer)
            } else {
                Ok(Self {
                    hdr,
                    _ref: PhantomData,
                })
            }
        }
    }
    pub unsafe fn from_bytes_unchecked(bytes: &'a [u8]) -> Self {
        Self {
            hdr: bytes.as_ptr().cast(),
            _ref: PhantomData,
        }
    }

    pub fn program(&self) -> ProgramTable<'a> {
        let hdr = self.raw();
        unsafe {
            ProgramTable::new(
                (self.hdr as *const u8).add(hdr.phoff as _),
                hdr.phentsize as _,
                hdr.phnum as _,
            )
        }
    }

    pub fn sections(&self) -> SectionTable<'a> {
        let hdr = self.raw();
        let shstrt = self.hdr as usize
            + hdr.shoff as usize
            + (hdr.shstrndx as usize) * (hdr.shentsize as usize);
        let shstrt = shstrt as *const raw::Elf64Shdr;
        unsafe {
            let shstrt = &*(shstrt);
            SectionTable::new(
                self.hdr as *const u8,
                (self.hdr as *const u8).add(hdr.shoff as usize),
                hdr.shentsize as _,
                hdr.shnum as _,
                StringTable::new(
                    (self.hdr as *const u8).offset(shstrt.off as _),
                    shstrt.size as _,
                ),
            )
        }
    }

    pub fn raw(&self) -> &'a raw::Elf64Hdr {
        unsafe { &*self.hdr }
    }

    pub unsafe fn data_ptr(&self, off: usize) -> *const u8 {
        let data = self.hdr as *const u8;
        data.add(off)
    }
}
