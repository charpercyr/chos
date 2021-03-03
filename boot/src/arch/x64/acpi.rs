
use core::fmt;
use core::marker::PhantomData;
use core::mem::{size_of, transmute};
use core::num::Wrapping;
use core::slice;

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

impl SDTHeader {
    pub fn is_checksum_valid(&self) -> bool {
        let ptr = self as *const Self as *const u8;
        let len = self.length as usize;
        let bytes = unsafe { slice::from_raw_parts(ptr, len) };
        let mut sum = Wrapping(0u8);
        for &b in bytes {
            sum += Wrapping(b);
        }
        sum == Wrapping(0)
    }
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
                .field("oem_table_id", &core::str::from_utf8_unchecked(&self.oem_table_id))
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

    pub fn madt(&self) -> Option<&MADT> {
        self.sdts().find(|&sdt| &sdt.sig == MADT::SIGNATURE).map(|sdt| unsafe { transmute(sdt) })
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

#[derive(Debug, Clone, Copy)]
#[repr(C, packed)]
pub struct MADT {
    pub hdr: SDTHeader,
    pub lapic_address: u32,
    pub flags: u32,
}

impl MADT {
    pub const SIGNATURE: &'static [u8; 4] = b"APIC";

    pub fn entries(&self) -> MADTIter<'_> {
        unsafe {
            let ptr = self as *const Self as *const u8;
            let ptr = ptr.offset(size_of::<Self>() as isize);
            let len = self.hdr.length as usize - size_of::<Self>();
            MADTIter {
                cur: ptr,
                end: ptr.offset(len as isize),
                madt: PhantomData,
            }
        }
    }
}

#[derive(Debug, Clone, Copy)]
#[repr(C, packed)]
pub struct MADTEntryHeader {
    pub typ: u8,
    pub len: u8,
}

#[derive(Debug, Clone, Copy)]
#[repr(C, packed)]
pub struct LAPICEntry {
    pub hdr: MADTEntryHeader,
    pub acpi_processor_id: u8,
    pub apic_id: u8,
    pub flags: u32,
}

#[derive(Debug, Clone, Copy)]
#[repr(C, packed)]
pub struct IOAPICEntry {
    pub hdr: MADTEntryHeader,
    pub ioapic_id: u8,
    _res0: u8,
    pub ioapic_address: u32,
    pub global_system_interrupt_base: u32,
}

#[derive(Debug, Clone, Copy)]
#[repr(C, packed)]
pub struct InterruptSourceOverrideEntry {
    pub hdr: MADTEntryHeader,
    pub bus_source: u8,
    pub irq_source: u8,
    pub global_system_interrupt: u32,
    pub flags: u16,
}

#[derive(Debug, Clone, Copy)]
#[repr(C, packed)]
pub struct NMIEntry {
    pub hdr: MADTEntryHeader,
    acpi_processor_id: u8,
    flags: u16,
    lint: u8,
}

#[derive(Debug, Clone, Copy)]
#[repr(C, packed)]
pub struct LAPICAddressOverrideEntry {
    pub hdr: MADTEntryHeader,
    pub address: u64,
}

#[derive(Debug, Clone, Copy)]
pub enum MADTEntry<'a> {
    LAPIC(&'a LAPICEntry),
    IOAPIC(&'a IOAPICEntry),
    InterruptSourceOverride(&'a InterruptSourceOverrideEntry),
    NMI(&'a NMIEntry),
    LAPICAddressOverride(&'a LAPICAddressOverrideEntry),
    Unknown(&'a MADTEntryHeader),
}

pub struct MADTIter<'a> {
    cur: *const u8,
    end: *const u8,
    madt: PhantomData<&'a MADT>,
}

impl<'a> Iterator for MADTIter<'a> {
    type Item = MADTEntry<'a>;
    fn next(&mut self) -> Option<Self::Item> {
        if self.cur != self.end {
            unsafe {
                let hdr = &*(self.cur as *const MADTEntryHeader);
                self.cur = self.cur.offset(hdr.len as isize);
                match hdr.typ {
                    0 => Some(MADTEntry::LAPIC(transmute(hdr))),
                    1 => Some(MADTEntry::IOAPIC(transmute(hdr))),
                    2 => Some(MADTEntry::InterruptSourceOverride(transmute(hdr))),
                    4 => Some(MADTEntry::NMI(transmute(hdr))),
                    5 => Some(MADTEntry::LAPICAddressOverride(transmute(hdr))),
                    _ => Some(MADTEntry::Unknown(hdr)),
                }
            }
        } else {
            None
        }
    }
}
