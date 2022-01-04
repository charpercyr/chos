mod dynamic;
pub use dynamic::*;

mod gnu_hash;
pub use gnu_hash::*;

mod macros;

mod program;
pub use program::*;

pub mod raw;

mod rela;
pub use rela::*;

mod section;
pub use section::*;

mod strtab;
pub use strtab::*;

mod symtab;
use core::mem::size_of;
use core::usize;

pub use symtab::*;

use self::raw::Elf64Hdr;

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum ElfErrorKind {
    InvalidSize,
    InvalidSignature,
}

#[derive(Copy, Clone, Debug)]
pub struct ElfError {
    pub kind: ElfErrorKind,
    pub msg: &'static str,
}

impl ElfError {
    pub const fn new(kind: ElfErrorKind, msg: &'static str) -> Self {
        Self { kind, msg }
    }
}

pub struct Elf<'a> {
    hdr: &'a raw::Elf64Hdr,
    data: &'a [u8],
}

impl<'a> Elf<'a> {
    pub fn new(data: &'a [u8]) -> Result<Self, ElfError> {
        if data.len() < size_of::<Elf64Hdr>() {
            return Err(ElfError::new(
                ElfErrorKind::InvalidSize,
                "Buffer is too small for header",
            ));
        }
        let hdr: *const Elf64Hdr = data.as_ptr().cast();
        let hdr = unsafe { &*hdr };
        if hdr.ident.magic != raw::MAGIC {
            return Err(ElfError::new(
                ElfErrorKind::InvalidSignature,
                "ELF signature is invalid",
            ));
        }

        if hdr.phoff as usize + (hdr.phentsize as usize * hdr.phnum as usize) > data.len() {
            return Err(ElfError::new(
                ElfErrorKind::InvalidSize,
                "Buffer is too small for program header",
            ));
        }

        if hdr.shoff as usize + (hdr.shentsize as usize * hdr.shnum as usize) > data.len() {
            return Err(ElfError::new(
                ElfErrorKind::InvalidSize,
                "Buffer is too small for section header",
            ));
        }

        Ok(Self { hdr, data })
    }

    pub fn get_buffer(&self, offset: usize, len: usize) -> &'a [u8] {
        &self.data[offset..(offset + len)]
    }

    pub fn program(&self) -> Program<'a> {
        unsafe {
            Program::new(
                self.get_buffer(
                    self.hdr.phoff as usize,
                    self.hdr.phnum as usize * self.hdr.phentsize as usize,
                ),
                self.hdr.phentsize as usize,
            )
        }
    }

    pub fn sections(&'a self) -> Sections<'a> {
        unsafe {
            Sections::new(
                self.get_buffer(
                    self.hdr.shoff as usize,
                    self.hdr.shnum as usize * self.hdr.shentsize as usize,
                ),
                self.hdr.shentsize as usize,
            )
        }
    }

    pub fn raw(&self) -> &Elf64Hdr {
        self.hdr
    }

    pub fn data(&self) -> &[u8] {
        self.data
    }
}
