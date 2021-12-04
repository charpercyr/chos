pub mod fadt;
pub mod hpet;
pub mod madt;
pub mod mcfg;

use core::fmt;
use core::marker::PhantomData;
use core::mem::{size_of, transmute};

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

#[derive(Debug, Clone, Copy)]
#[repr(C, packed)]
pub struct Rsdt {
    pub hdr: SDTHeader,
}

impl Rsdt {
    pub fn sdts(&self) -> RSDTIter {
        let (ptr, len) = self.sdt_ptr();
        RSDTIter {
            cur: ptr,
            end: unsafe { ptr.add(len) },
            rsdt: PhantomData,
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
        self.sdts().find(|&sdt| &sdt.sig == sig).map(|hdr| unsafe { transmute(hdr) })
    }

    fn sdt_ptr(&self) -> (*const u32, usize) {
        unsafe {
            let ptr = (self as *const Self).offset(1) as *const u8;
            let byte_len = self.hdr.length as usize - size_of::<Self>();
            assert!(byte_len % size_of::<u32>() == 0);
            (ptr as _, byte_len / size_of::<u32>())
        }
    }
}

pub struct RSDTIter<'a> {
    cur: *const u32,
    end: *const u32,
    rsdt: PhantomData<&'a Rsdt>,
}

impl<'a> Iterator for RSDTIter<'a> {
    type Item = &'a SDTHeader;
    fn next(&mut self) -> Option<Self::Item> {
        if self.cur != self.end {
            unsafe {
                let hdr = &*((*self.cur) as *const SDTHeader);
                self.cur = self.cur.offset(1);
                Some(hdr)
            }
        } else {
            None
        }
    }
}