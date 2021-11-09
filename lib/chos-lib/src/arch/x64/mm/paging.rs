
use core::{ops::{Index, IndexMut}, slice::{Iter, IterMut}};

use modular_bitfield::prelude::*;

use crate::mm::FrameSize;

use super::{PAddr};

pub const PAGE_TABLE_SIZE: usize = 512;

#[bitfield(bits = 64)]
#[derive(Clone, Copy, Debug)]
pub struct PageEntry {
    pub present: bool,
    pub writable: bool,
    pub user: bool,
    pub write_through: bool,
    pub no_cache: bool,
    pub accessed: bool,
    pub dirty: bool,
    pub huge_page: bool,
    pub global: bool,
    pub os0: B3,
    addr: B40,
    pub os1: B11,
    pub no_execute: bool,
}

impl PageEntry {
    pub const fn zero() -> Self {
        Self::new()
    }

    pub fn phys_addr(&self) -> PAddr {
        PAddr::new(self.addr() << 12)
    }

    pub fn set_phys_addr(&mut self, addr: PAddr) {
        assert!(addr.is_page_aligned(), "Address is not page aligned");
        self.set_addr(addr.page());
    }

    pub fn with_phys_addr(self, addr: PAddr) -> Self {
        assert!(addr.is_page_aligned(), "Address is not page aligned");
        self.with_addr(addr.page())
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

#[derive(Clone, Copy, Debug)]
pub struct FrameSize4K;
impl FrameSize for FrameSize4K {
    const PAGE_SIZE: u64 = 4096;
    const DEBUG_STR: &'static str = "4K";
}

#[derive(Clone, Copy, Debug)]
pub struct FrameSize2M;
impl FrameSize for FrameSize2M {
    const PAGE_SIZE: u64 = 512 * 4096;
    const DEBUG_STR: &'static str = "2M";
}

#[derive(Clone, Copy, Debug)]
pub struct FrameSize1G;
impl FrameSize for FrameSize1G {
    const PAGE_SIZE: u64 = 512 * 512 * 4096;
    const DEBUG_STR: &'static str = "1G";
}
