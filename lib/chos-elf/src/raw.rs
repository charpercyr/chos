use core::u32;

pub const MAGIC: [u8; 4] = [0x7f, b'E', b'L', b'F'];

pub const CLASS32: u8 = 1;
pub const CLASS64: u8 = 2;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(C, packed)]
pub struct Elf64Ident {
    pub magic: [u8; 4],
    pub class: u8,
    pub data: u8,
    pub version: u8,
    pub osabi: u8,
    pub abiversion: u8,
    pub _pad: [u8; 7],
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(C, packed)]
pub struct Elf64Hdr {
    pub ident: Elf64Ident,
    pub typ: u16,
    pub machine: u16,
    pub version: u32,
    pub entry: u64,
    pub phoff: u64,
    pub shoff: u64,
    pub flags: u32,
    pub ehsize: u16,
    pub phentsize: u16,
    pub phnum: u16,
    pub shentsize: u16,
    pub shnum: u16,
    pub shstrndx: u16,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(C, packed)]
pub struct Elf64Shdr {
    pub name: u32,
    pub typ: u32,
    pub flags: u64,
    pub addr: u64,
    pub off: u64,
    pub size: u64,
    pub link: u32,
    pub info: u32,
    pub addr_align: u64,
    pub entsize: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(C, packed)]
pub struct Elf64Sym {
    pub name: u32,
    pub info: u8,
    pub other: u8,
    pub shndx: u16,
    pub value: u64,
    pub size: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(C, packed)]
pub struct Elf64Phdr {
    pub typ: u32,
    pub flags: u32,
    pub off: u64,
    pub vaddr: u64,
    pub paddr: u64,
    pub filesz: u64,
    pub memsz: u64,
    pub align: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(C, packed)]
pub struct Elf64Dyn {
    pub tag: u64,
    pub val: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(C, packed)]
pub struct Elf64Rela {
    pub off: u64,
    pub info: u64,
    pub addend: i64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(C, packed)]
pub struct Elf64GnuHash {
    pub nbuckets: u32,
    pub symoffset: u32,
    pub bloom_size: u32,
    pub bloom_shift: u32,
}
