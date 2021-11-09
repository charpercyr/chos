use core::mem::size_of;
use core::ptr::write;

use chos_lib::arch::mm::FrameSize4K;
use chos_lib::arch::x64::mm::{PAddr, PageTable, VAddr, PAGE_SIZE, PAGE_SIZE64};
use chos_lib::log::info;
use chos_lib::mm::*;

use super::mapper::BootMapper;

pub struct PAlloc {
    pbase: *mut PageTable,
    pcur: *mut PageTable,
}

unsafe impl FrameAllocator for PAlloc {
    type Error = !;
    unsafe fn alloc_frame<S: FrameSize>(&mut self) -> Result<VFrame<S>, !> {
        let ptr = self.pcur;
        self.pcur = self.pcur.add(1);
        write(ptr, PageTable::empty());
        Ok(VFrame::new_unchecked(VAddr::new_unchecked(ptr as u64)))
    }

    unsafe fn dealloc_frame<S: FrameSize>(&mut self, _: VFrame<S>) -> Result<(), !> {
        panic!("Cannot dealloc with this deallocator")
    }
}

impl PAlloc {
    pub unsafe fn new(pbase: *mut u8) -> Self {
        assert_eq!(
            pbase as usize % PAGE_SIZE,
            0,
            "Physical base must be aligned to a physical page boundary"
        );
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

    pub unsafe fn map_self(&mut self, mut vaddr: VAddr, mapper: &mut BootMapper) {
        let mut cur = self.pbase;
        // We might need to allocate more pages, so self.pcur might change during iteration
        while cur < self.pcur {
            info!("Map PGT {:016x} -> {:016x}", cur as u64, vaddr.as_u64());
            mapper
                .mapper
                .map(
                    PFrame::<FrameSize4K>::new_unchecked(PAddr::new(cur as u64)),
                    VFrame::new_unchecked(vaddr),
                    MapFlags::WRITE,
                    self,
                )
                .unwrap()
                .ignore();
            cur = cur.add(1);
            vaddr = VAddr::try_new(vaddr.as_u64() + PAGE_SIZE64)
                .expect("Got invalid vaddr, this is very unlikely");
        }
    }

    pub fn total_size(&self) -> usize {
        unsafe { (self.pcur.offset_from(self.pbase) as usize) * size_of::<PageTable>() }
    }
}
