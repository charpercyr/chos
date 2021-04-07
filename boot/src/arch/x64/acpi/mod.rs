
pub mod hpet;
pub mod madt;

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
        unsafe {
            f.debug_struct("SDTHeader")
                .field("sig", &core::str::from_utf8_unchecked(&self.sig))
                .field("length", &self.length)
                .field("revision", &self.revision)
                .field("checksum", &self.checksum)
                .field("oemid", &core::str::from_utf8_unchecked(&self.oemid))
                .field(
                    "oem_table_id",
                    &core::str::from_utf8_unchecked(&self.oem_table_id),
                )
                .field("oem_revision", &self.oem_revision)
                .field("creator_id", &self.creator_id)
                .field("creator_revision", &self.creator_revision)
                .finish()
        }
    }
}

#[derive(Debug, Clone, Copy)]
#[repr(C, packed)]
pub struct RSDT {
    pub hdr: SDTHeader,
}

impl RSDT {
    pub fn sdts(&self) -> RSDTIter {
        let (ptr, len) = self.sdt_ptr();
        RSDTIter {
            cur: ptr,
            end: unsafe { ptr.offset(len as _) },
            rsdt: PhantomData,
        }
    }

    pub fn madt(&self) -> Option<&madt::MADT> {
        self.find_table(madt::MADT::SIGNATURE).map(|hdr| unsafe { transmute(hdr) })
    }

    pub fn hpet(&self) -> Option<&hpet::HPET> {
        self.find_table(hpet::HPET::SIGNATURE).map(|hdr| unsafe { transmute(hdr) })
    }

    fn find_table(&self, sig: &[u8; 4]) -> Option<&SDTHeader> {
        self.sdts()
            .find(|&sdt| &sdt.sig == sig)
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
    rsdt: PhantomData<&'a RSDT>,
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
