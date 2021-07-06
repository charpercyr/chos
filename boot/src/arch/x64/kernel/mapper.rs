
use chos_x64::paging::{PAddr, PageEntry, PageTable, VAddr};

use super::palloc::PAlloc;

pub struct Mapper {
    pub p4: &'static mut PageTable,
}

fn init_page_entry(e: &mut PageEntry, paddr: PAddr, write: bool, exec: bool) {
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

    pub unsafe fn map(&mut self, paddr: PAddr, vaddr: VAddr, write: bool, exec: bool, alloc: &mut PAlloc) {
        assert!(paddr.is_page_aligned(), "Physical address must be page aligned");
        assert!(vaddr.is_page_aligned(), "Virtual address must be page aligned");
        let (p4i, p3i, p2i, p1i, _) = vaddr.split();

        let p3 = &mut self.p4[p4i];
        let p3 = if !p3.present() {
            let p3 = alloc.alloc_page_table();
            init_page_entry(&mut self.p4[p4i], PAddr::new(p3 as *mut _ as u64), true, true);
            p3
        } else {
            &mut *(p3.phys_addr().as_u64() as *mut PageTable)
        };

        let p2 = &mut p3[p3i];
        let p2 = if !p2.present() {
            let p2 = alloc.alloc_page_table();
            init_page_entry(&mut p3[p3i], PAddr::new(p2 as *mut _ as u64), true, true);
            p2
        } else {
            &mut *(p2.phys_addr().as_u64() as *mut PageTable)
        };

        let p1 = &mut p2[p2i];
        let p1 = if !p1.present() {
            let p1 = alloc.alloc_page_table();
            init_page_entry(&mut p2[p2i], PAddr::new(p1 as *mut _ as u64), true, true);
            p1
        } else {
            &mut *(p1.phys_addr().as_u64() as *mut PageTable)
        };

        init_page_entry(&mut p1[p1i], paddr, write, exec);
    }

    pub unsafe fn identity_map_4g(&mut self, alloc: &mut PAlloc) {
        let p3 = alloc.alloc_page_table();
        init_page_entry(&mut self.p4[0], PAddr::new(p3 as *mut _ as u64), true, true);
        init_page_entry(&mut p3[0], PAddr::new(0x0000_0000), true, true);
        init_page_entry(&mut p3[1], PAddr::new(0x4000_0000), true, true);
        init_page_entry(&mut p3[2], PAddr::new(0x8000_0000), true, true);
        init_page_entry(&mut p3[3], PAddr::new(0xc000_0000), true, true);
        
        p3[0].set_huge_page(true);
        p3[1].set_huge_page(true);
        p3[2].set_huge_page(true);
        p3[3].set_huge_page(true);
    }
}
