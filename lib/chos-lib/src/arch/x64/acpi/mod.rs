pub mod fadt;
pub mod hpet;
pub mod madt;
pub mod mcfg;

use core::fmt;
use core::marker::PhantomData;
use core::mem::{size_of, transmute};

use crate::mm::VAddr;

#[derive(Clone, Copy)]
#[repr(C, packed)]
pub struct SDTHeader {
    pub sig: [u8; 4],
    pub length: u32,
    pub revision: u8,
    pub checksum: u8,
    pub oemid: [u8; 6],
    pub oem_table_id: [u8; 8],
    pub oem_revision: u32,
    pub creator_id: u32,
    pub creator_revision: u32,
}

impl fmt::Debug for SDTHeader {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let Self {
            sig,
            length,
            revision,
            checksum,
            oemid,
            oem_table_id,
            oem_revision,
            creator_id,
            creator_revision,
        } = *self;
        unsafe {
            f.debug_struct("SDTHeader")
                .field("sig", &core::str::from_utf8_unchecked(&sig))
                .field("length", &length)
                .field("revision", &revision)
                .field("checksum", &checksum)
                .field("oemid", &core::str::from_utf8_unchecked(&oemid))
                .field(
                    "oem_table_id",
                    &core::str::from_utf8_unchecked(&oem_table_id),
                )
                .field("oem_revision", &oem_revision)
                .field("creator_id", &creator_id)
                .field("creator_revision", &creator_revision)
                .finish()
        }
    }
}

#[derive(Clone, Copy)]
pub struct Rsdt<'a> {
    offset: VAddr,
    addr: usize,
    rsdt: PhantomData<&'a SDTHeader>,
}

impl<'a> Rsdt<'a> {
    pub unsafe fn new(addr: usize) -> Self {
        Self::new_offset(addr, VAddr::null())
    }

    pub unsafe fn new_offset(addr: usize, offset: VAddr) -> Self {
        Self {
            offset,
            addr,
            rsdt: PhantomData,
        }
    }

    pub fn hdr(&self) -> &SDTHeader {
        unsafe { (self.offset + self.addr as u64).as_ref() }
    }

    pub fn tables(&self) -> Iter<'a> {
        unsafe {
            let hdr: *const SDTHeader = (self.offset + self.addr as u64).as_ptr();
            let len = (*hdr).length as usize - size_of::<SDTHeader>();
            let ptr = hdr.add(1).cast();
            Iter {
                cur: ptr,
                end: ptr.add(len / size_of::<u32>()),
                base: self.offset,
                rsdt: PhantomData,
            }
        }
    }

    pub fn madt(&self) -> Option<&madt::Madt> {
        unsafe { self.find_table(madt::Madt::SIGNATURE) }
    }

    pub fn hpet(&self) -> Option<&hpet::Hpet> {
        unsafe { self.find_table(hpet::Hpet::SIGNATURE) }
    }

    pub fn mcfg(&self) -> Option<&mcfg::Mcfg> {
        unsafe { self.find_table(mcfg::Mcfg::SIGNATURE) }
    }

    pub fn fadt(&self) -> Option<&fadt::Fadt> {
        unsafe { self.find_table(fadt::Fadt::SIGNATURE) }
    }

    unsafe fn find_table<T>(&self, sig: &[u8; 4]) -> Option<&T> {
        self.tables()
            .find(|&sdt| &sdt.sig == sig)
            .map(|hdr| transmute(hdr))
    }
}

pub struct Iter<'a> {
    cur: *const u32,
    end: *const u32,
    base: VAddr,
    rsdt: PhantomData<&'a Rsdt<'a>>,
}
impl<'a> Iterator for Iter<'a> {
    type Item = &'a SDTHeader;
    fn next(&mut self) -> Option<Self::Item> {
        (self.cur != self.end).then(|| unsafe {
            let hdr = (self.base + (*self.cur) as u64).as_ref();
            self.cur = self.cur.add(1);
            hdr
        })
    }
}

impl<'a> IntoIterator for &'a Rsdt<'a> {
    type Item = &'a SDTHeader;
    type IntoIter = Iter<'a>;
    fn into_iter(self) -> Self::IntoIter {
        self.tables()
    }
}
