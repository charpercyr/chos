
use core::mem::transmute;
use core::ops::{Index, IndexMut};

use chos_lib::{Either, bitfield::*};

pub const PAGE_TABLE_SIZE: usize = 512;
pub const PAGE_SIZE: usize = 4096;

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

    pub fn phys_addr(&self) -> u64 {
        self.addr() << 12
    }
    
    pub fn set_phys_addr(&mut self, addr: u64) -> &mut Self {
        assert_eq!(addr % 4096, 0);
        self.set_addr(addr >> 12)
    }
}

#[derive(Clone, Debug)]
pub struct PageTable {
    entries: [PageEntry; PAGE_TABLE_SIZE],
}

impl PageTable {
    pub const fn empty() -> Self {
        Self {
            entries: [PageEntry::zero(); PAGE_TABLE_SIZE],
        }
    }
}

impl Index<usize> for PageTable {
    type Output = PageEntry;

    fn index(&self, index: usize) -> &Self::Output {
        &self.entries[index]
    }
}

impl IndexMut<usize> for PageTable {
    fn index_mut(&mut self, index: usize) -> &mut Self::Output {
        &mut self.entries[index]
    }
}

pub fn split_virtual_address(addr: u64) -> Option<(u16, u16, u16, u16, u16)> {
    if (addr & (1 << 47)) != 0 && (addr & 0xffff_0000_0000_0000) != 0xffff_0000_0000_0000 {
        return None;
    } else if (addr & (1 << 47)) == 0 && (addr & 0xffff_0000_0000_0000) != 0 {
        return None;
    }
    let off = (addr & 0xfff) as u16;
    let p1 = ((addr & (0x1ff << 12)) >> 12) as u16;
    let p2 = ((addr & (0x1ff << 21)) >> 21) as u16;
    let p3 = ((addr & (0x1ff << 30)) >> 30) as u16;
    let p4 = ((addr & (0x1ff << 39)) >> 39) as u16;
    Some((p4, p3, p2, p1, off))
}

pub fn make_canonical(addr: u64) -> u64 {
    if addr & (1 << 47) != 0 {
        addr | 0xffff_0000_0000_0000
    } else {
        addr & 0x0000_ffff_ffff_ffff
    }
}