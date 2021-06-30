
use core::ptr::write;

use chos_x64::paging::{PAGE_SIZE, PageTable};

use super::mapper::Mapper;

pub struct PAlloc {
    pbase: *mut PageTable,
    pcur: *mut PageTable,
}

impl PAlloc {
    pub unsafe fn new(pbase: *mut u8) -> Self {
        assert_eq!(pbase as usize % PAGE_SIZE, 0, "Physical base must be aligned to a physical page boundary");
        Self {
            pbase: pbase.cast(),
            pcur: pbase.cast(),
        }
    }

    pub fn alloc_page_table(&mut self) -> &'static mut PageTable {
        unsafe {
            let ptr = self.pcur;
            self.pcur = self.pcur.add(1);
            write(ptr, PageTable::empty());
            &mut *ptr
        }
    }

    pub unsafe fn map_self(&mut self, mut vaddr: u64, mapper: &mut Mapper) {
        let mut cur = self.pbase;
        // We might need to allocate more pages, so self.pcur might change during iteration
        while cur < self.pcur {
            mapper.map(cur as u64, vaddr, true, false, self);
            cur = cur.add(1);
            vaddr += PAGE_SIZE as u64;
        }
    }
}
