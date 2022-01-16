use core::alloc::AllocError;

use chos_config::arch::mm::virt;
use chos_lib::arch::mm::{FrameSize4K, PAddr, PageTable, VAddr};
use chos_lib::mm::{FrameAllocator, PFrame, VFrame};

use crate::arch::early::copy_early_kernel_table_to;
use crate::mm::phys::{raw_alloc, AllocFlags};
use crate::mm::PerCpu;
use crate::mm::virt::{paddr_of, MemoryRegion};
use crate::per_cpu;

pub struct MMFrameAllocator;

unsafe impl FrameAllocator<FrameSize4K> for MMFrameAllocator {
    type Error = AllocError;
    unsafe fn alloc_frame(&mut self) -> Result<VFrame<FrameSize4K>, Self::Error> {
        raw_alloc::alloc_pages(0, AllocFlags::empty())
            .map(|p| VFrame::new_unchecked(p + virt::PHYSICAL_MAP_BASE))
    }
    unsafe fn dealloc_frame(&mut self, frame: VFrame<FrameSize4K>) -> Result<(), Self::Error> {
        raw_alloc::dealloc_pages(
            PAddr::new((frame.addr() - virt::PHYSICAL_MAP_BASE).as_u64()),
            0,
        );
        Ok(())
    }
}

per_cpu! {
    static mut ref PAGE_TABLE: PageTable = PageTable::empty();
}

pub unsafe fn init_kernel_virt() {
    PAGE_TABLE.with(|pgt| {
        copy_early_kernel_table_to(pgt);
        let vaddr = VAddr::from(pgt);
        let paddr = paddr_of(vaddr, MemoryRegion::PerCpu).expect("PerCpu should be mapped");
        PageTable::set_page_table(PFrame::new(paddr));
    });
}
