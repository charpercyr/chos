use core::marker::PhantomData;
use core::mem::size_of;

use super::SDTHeader;

#[derive(Debug, Clone, Copy)]
#[repr(C)]
pub struct Mcfg {
    pub hdr: SDTHeader,
}

impl Mcfg {
    pub const SIGNATURE: &'static [u8; 4] = b"MCFG";

    pub fn iter(&self) -> Iter<'_> {
        unsafe {
            let ptr = self as *const Self as *const u8;
            let end = ptr.add(self.hdr.length as usize);
            let ptr = ptr.add(size_of::<Self>());
            Iter {
                cur: ptr,
                end,
                mcfg: PhantomData,
            }
        }
    }
}

impl<'a> IntoIterator for &'a Mcfg {
    type Item = &'a Entry;
    type IntoIter = Iter<'a>;

    fn into_iter(self) -> Self::IntoIter {
        self.iter()
    }
}

#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct Entry {
    pub base: u64,
    pub segment_group_number: u16,
    pub start_pci_bus_number: u8,
    pub end_pci_bus_number: u8,
    _res0: u32,
}

pub struct Iter<'a> {
    cur: *const u8,
    end: *const u8,
    mcfg: PhantomData<&'a Mcfg>,
}

impl<'a> Iterator for Iter<'a> {
    type Item = &'a Entry;

    fn next(&mut self) -> Option<Self::Item> {
        if self.cur < self.end {
            unsafe {
                let ptr = self.cur.cast();
                self.cur = self.cur.add(size_of::<Entry>());
                Some(&*ptr)
            }
        } else {
            None
        }
    }
}
