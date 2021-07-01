use core::marker::PhantomData;
use core::mem::{size_of, transmute};

use bitflags::bitflags;

use super::SDTHeader;

#[derive(Debug, Clone, Copy)]
#[repr(C, packed)]
pub struct MADT {
    pub hdr: SDTHeader,
    pub lapic_address: u32,
    pub flags: u32,
}

impl MADT {
    pub const SIGNATURE: &'static [u8; 4] = b"APIC";

    pub fn entries(&self) -> Iter<'_> {
        unsafe {
            let ptr = self as *const Self as *const u8;
            let ptr = ptr.offset(size_of::<Self>() as isize);
            let len = self.hdr.length as usize - size_of::<Self>();
            Iter {
                cur: ptr,
                end: ptr.offset(len as isize),
                madt: PhantomData,
            }
        }
    }

    pub fn apic_count(&self) -> usize {
        self.entries()
            .filter(|e| if let Entry::LAPIC(_) = e { true } else { false })
            .count()
    }
}

#[derive(Debug, Clone, Copy)]
#[repr(C, packed)]
pub struct EntryHeader {
    pub typ: u8,
    pub len: u8,
}

#[derive(Debug, Clone, Copy)]
#[repr(C, packed)]
pub struct LAPICEntry {
    pub hdr: EntryHeader,
    pub acpi_processor_id: u8,
    pub apic_id: u8,
    pub flags: u32,
}

#[derive(Debug, Clone, Copy)]
#[repr(C, packed)]
pub struct IOAPICEntry {
    pub hdr: EntryHeader,
    pub ioapic_id: u8,
    _res0: u8,
    pub ioapic_address: u32,
    pub global_system_interrupt_base: u32,
}

#[derive(Debug, Clone, Copy)]
#[repr(C, packed)]
pub struct InterruptSourceOverrideEntry {
    pub hdr: EntryHeader,
    pub bus_source: u8,
    pub irq_source: u8,
    pub global_system_interrupt: u32,
    flags: u16,
}

bitflags! {
    pub struct InterruptSourceOverrideFlags: u16 {
        const ACTIVE_LOW = 1 << 2;
        const LEVEL_TRIGGERED = 1 << 8;
    }
}

#[derive(Debug, Clone, Copy)]
#[repr(C, packed)]
pub struct NMIEntry {
    pub hdr: EntryHeader,
    acpi_processor_id: u8,
    flags: u16,
    lint: u8,
}

#[derive(Debug, Clone, Copy)]
#[repr(C, packed)]
pub struct LAPICAddressOverrideEntry {
    pub hdr: EntryHeader,
    pub address: u64,
}

#[derive(Debug, Clone, Copy)]
pub enum Entry<'a> {
    LAPIC(&'a LAPICEntry),
    IOAPIC(&'a IOAPICEntry),
    InterruptSourceOverride(&'a InterruptSourceOverrideEntry),
    NMI(&'a NMIEntry),
    LAPICAddressOverride(&'a LAPICAddressOverrideEntry),
    Unknown(&'a EntryHeader),
}

#[derive(Clone, Copy)]
pub struct Iter<'a> {
    cur: *const u8,
    end: *const u8,
    madt: PhantomData<&'a MADT>,
}

impl<'a> Iterator for Iter<'a> {
    type Item = Entry<'a>;
    fn next(&mut self) -> Option<Self::Item> {
        if self.cur != self.end {
            unsafe {
                let hdr = &*(self.cur as *const EntryHeader);
                self.cur = self.cur.offset(hdr.len as isize);
                match hdr.typ {
                    0 => Some(Entry::LAPIC(transmute(hdr))),
                    1 => Some(Entry::IOAPIC(transmute(hdr))),
                    2 => Some(Entry::InterruptSourceOverride(transmute(hdr))),
                    4 => Some(Entry::NMI(transmute(hdr))),
                    5 => Some(Entry::LAPICAddressOverride(transmute(hdr))),
                    _ => Some(Entry::Unknown(hdr)),
                }
            }
        } else {
            None
        }
    }
}
