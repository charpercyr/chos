
use core::fmt;
use core::ops::{Index, IndexMut};
use core::slice::{Iter, IterMut};

use chos_lib::bitfield::*;

pub const PAGE_TABLE_SIZE: usize = 512;

pub const PAGE_SHIFT: u32 = 12;
pub const PAGE_MASK: u64 = (1 << PAGE_SHIFT) - 1;
pub const PAGE_SIZE: usize = 1 << PAGE_SHIFT;
pub const PAGE_SIZE64: u64 = 1 << PAGE_SHIFT;

macro_rules! impl_addr_fns {
    ($ty:ty) => {
        impl $ty {
            pub const fn as_u64(self) -> u64 {
                self.0
            }
        
            pub const fn is_page_aligned(self) -> bool {
                self.0 & PAGE_MASK == 0
            }
        
            pub const fn align_page(self) -> Self {
                Self(self.0 & !PAGE_MASK)
            }
        
            pub const fn align_page_up(self) -> Self {
                Self((self.0 + PAGE_SIZE64 - 1) / PAGE_SIZE64 * PAGE_SIZE64)
            }
        
            pub const fn page(self) -> u64 {
                self.0 >> 12
            }

            pub unsafe fn offset(self, o: i64) -> Self {
                if o < 0 {
                    Self(self.0 - (-o as u64))
                } else {
                    Self(self.0 + o as u64)
                }
            }

            pub unsafe fn add(self, o: u64) -> Self {
                Self(self.0 + o)
            }

            pub unsafe fn sub(self, o: u64) -> Self {
                Self(self.0 - o)
            }
        }

        impl fmt::Debug for $ty {
            fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
                write!(f, concat!(stringify!($ty), "({:#x})"), self.0)
            }
        }

        impl fmt::Pointer for $ty {
            fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
                write!(f, "{:#x}", self.0)
            }
        }
    };
}

#[repr(transparent)]
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
pub struct VAddr(u64);

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum VaddrError {
    NonCanonical,
}

impl VAddr {
    pub const fn new(v: u64) -> Result<Self, VaddrError> {
        if v & (1 << 47) != 0 && v & 0xffff_0000_0000_0000 != 0xffff_0000_0000_0000 {
            Err(VaddrError::NonCanonical)
        } else if (v & (1 << 47)) == 0 && v & 0xffff_0000_0000_0000 != 0 {
            Err(VaddrError::NonCanonical)
        } else {
            Ok(Self(v))
        }
    }

    pub const fn make_canonical(mut v: u64) -> Self {
        v |= if v & (1 << 47) != 0 { 0xffff_0000_0000_0000 } else { 0 };
        Self(v)
    }

    pub const unsafe fn new_unchecked(v: u64) -> Self {
        Self(v)
    }

    pub fn split(self) -> (u16, u16, u16, u16, u16) {
        let addr = self.0;
        let off = (addr & 0xfff) as u16;
        let p1 = ((addr & (0x1ff << 12)) >> 12) as u16;
        let p2 = ((addr & (0x1ff << 21)) >> 21) as u16;
        let p3 = ((addr & (0x1ff << 30)) >> 30) as u16;
        let p4 = ((addr & (0x1ff << 39)) >> 39) as u16;
        (p4, p3, p2, p1, off)
    }
}
impl_addr_fns!(VAddr);

#[repr(transparent)]
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
pub struct PAddr(u64);

impl PAddr {
    pub const fn new(v: u64) -> Self {
        Self(v)
    }
}
impl_addr_fns!(PAddr);

bitfield! {
    #[derive(Clone, Copy)]
    pub struct PageEntry (u64) {
        [vis pub]
        [imp Debug]
        no_execute, set_no_execute: 63;
        os1, set_os1: 62, 52 -> u16;
        pub(self) addr, pub(self) set_addr: 51, 12;
        os0, set_os0: 11, 9 -> u8;
        global, set_global: 8;
        huge_page, set_huge_page: 7;
        dirty, set_dirty: 6;
        accessed, set_accessed: 5;
        no_cache, set_no_cache: 4;
        write_trough, set_write_through: 3;
        user, set_user: 2;
        writable, set_writable: 1;
        present, set_present: 0;
    }
}

impl PageEntry {
    pub const fn zero() -> Self {
        Self::new(0)
    }

    pub fn phys_addr(&self) -> PAddr {
        PAddr::new(self.addr() << 12)
    }
    
    pub fn set_phys_addr(&mut self, addr: PAddr) -> &mut Self {
        assert!(addr.is_page_aligned(), "Address is not page aligned");
        self.set_addr(addr.page())
    }
}

#[derive(Clone, Debug)]
#[repr(C, align(4096))]
pub struct PageTable {
    entries: [PageEntry; PAGE_TABLE_SIZE],
}

impl PageTable {
    pub const fn empty() -> Self {
        Self {
            entries: [PageEntry::zero(); PAGE_TABLE_SIZE],
        }
    }

    pub fn iter(&self) -> PageTableIter<'_> {
        PageTableIter {
            iter: self.entries.iter(),
        }
    }

    pub fn iter_mut(&mut self) -> PageTableIterMut<'_> {
        PageTableIterMut {
            iter: self.entries.iter_mut(),
        }
    }

    pub unsafe fn set_page_table(&mut self) {
        asm! {
            "mov %rax, %cr3",
            in("rax") self,
            options(att_syntax, nostack),
        }
    }

    pub unsafe fn get_current_page_table() -> &'static mut Self {
        let pgt: *mut Self;
        asm! {
            "mov %cr3, %rax",
            lateout("rax") pgt,
            options(att_syntax, nostack, nomem),
        }
        &mut *pgt
    }
}

impl Index<u16> for PageTable {
    type Output = PageEntry;

    fn index(&self, index: u16) -> &Self::Output {
        &self.entries[index as usize]
    }
}

impl IndexMut<u16> for PageTable {
    fn index_mut(&mut self, index: u16) -> &mut Self::Output {
        &mut self.entries[index as usize]
    }
}

pub struct PageTableIter<'a> {
    iter: Iter<'a, PageEntry>,
}

impl<'a> Iterator for PageTableIter<'a> {
    type Item = &'a PageEntry;
    fn next(&mut self) -> Option<Self::Item> {
        self.iter.next()
    }
}

impl<'a> IntoIterator for &'a PageTable {
    type Item = &'a PageEntry;
    type IntoIter = PageTableIter<'a>;
    fn into_iter(self) -> Self::IntoIter {
        self.iter()
    }
}

impl<'a> IntoIterator for &'a mut PageTable {
    type Item = &'a mut PageEntry;
    type IntoIter = PageTableIterMut<'a>;
    fn into_iter(self) -> Self::IntoIter {
        self.iter_mut()
    }
}

pub struct PageTableIterMut<'a> {
    iter: IterMut<'a, PageEntry>,
}

impl<'a> Iterator for PageTableIterMut<'a> {
    type Item = &'a mut PageEntry;
    fn next(&mut self) -> Option<Self::Item> {
        self.iter.next()
    }
}

pub fn make_canonical(addr: u64) -> u64 {
    if (addr & (1 << 47)) != 0 {
        addr | 0xffff_0000_0000_0000
    } else {
        addr & 0x0000_ffff_ffff_ffff
    }
}