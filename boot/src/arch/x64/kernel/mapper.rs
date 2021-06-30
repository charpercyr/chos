
use chos_x64::paging::{PAGE_SIZE, PageEntry, PageTable, split_virtual_address};

use super::palloc::PAlloc;

pub struct Mapper {
    pub p4: &'static mut PageTable,
}

fn init_page_entry(e: &mut PageEntry, paddr: u64, write: bool, exec: bool) {
    e
        .set_no_execute(!exec)
        .set_phys_addr(paddr)
        .set_global(true)
        .set_writable(write)
        .set_present(true)
    ;
}

impl Mapper {
    pub unsafe fn new(alloc: &mut PAlloc) -> Self {
        let p4 = alloc.alloc_page_table();
        Self {
            p4: &mut *p4,
        }
    }

    pub unsafe fn map(&mut self, paddr: u64, vaddr: u64, write: bool, exec: bool, alloc: &mut PAlloc) {
        assert_eq!(paddr % PAGE_SIZE as u64, 0, "Physical address must be page aligned");
        assert_eq!(vaddr % PAGE_SIZE as u64, 0, "Virtual address must be page aligned");
        let (p4i, p3i, p2i, p1i, _) = split_virtual_address(vaddr).expect("Invalid virtual address");

        let p3 = &mut self.p4[p4i];
        let p3 = if !p3.present() {
            let p3 = alloc.alloc_page_table();
            init_page_entry(&mut self.p4[p4i], p3 as *mut _ as u64, true, true);
            p3
        } else {
            &mut *(p3.phys_addr() as *mut PageTable)
        };

        let p2 = &mut p3[p3i];
        let p2 = if !p2.present() {
            let p2 = alloc.alloc_page_table();
            init_page_entry(&mut p3[p3i], p2 as *mut _ as u64, true, true);
            p2
        } else {
            &mut *(p2.phys_addr() as *mut PageTable)
        };

        let p1 = &mut p2[p2i];
        let p1 = if !p1.present() {
            let p1 = alloc.alloc_page_table();
            init_page_entry(&mut p2[p2i], p1 as *mut _ as u64, true, true);
            p1
        } else {
            &mut *(p1.phys_addr() as *mut PageTable)
        };

        init_page_entry(&mut p1[p1i], paddr as u64, write, exec);
    }

    pub unsafe fn identity_map_4g(&mut self, alloc: &mut PAlloc) {
        let p3 = alloc.alloc_page_table();
        init_page_entry(&mut self.p4[0], p3 as *mut _ as u64, true, true);
        init_page_entry(&mut p3[0], 0x0000_0000, true, true);
        init_page_entry(&mut p3[1], 0x4000_0000, true, true);
        init_page_entry(&mut p3[2], 0x8000_0000, true, true);
        init_page_entry(&mut p3[3], 0xc000_0000, true, true);
        
        p3[0].set_huge_page(true);
        p3[1].set_huge_page(true);
        p3[2].set_huge_page(true);
        p3[3].set_huge_page(true);
    }
}
