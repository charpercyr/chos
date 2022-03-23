use core::alloc::AllocError;

use chos_config::arch::mm::virt;
use chos_lib::arch::mm::{FrameSize4K, OffsetMapper, PageTable};
use chos_lib::mm::{
    FrameAllocator, FrameSize, LoggingMapper, MapFlags, MapperFlush, PAddr, PFrame, PFrameRange,
    RangeMapper, VAddr, VFrame,
};

use crate::arch::early::{copy_early_kernel_table_to, early_paddr_of};
use crate::mm::phys::{raw_alloc, AllocFlags, Page};
use crate::mm::{per_cpu, per_cpu_lazy, PerCpu};

pub struct MMFrameAllocator;

unsafe impl FrameAllocator<FrameSize4K> for MMFrameAllocator {
    type Error = AllocError;
    unsafe fn alloc_frame(&mut self) -> Result<VFrame<FrameSize4K>, Self::Error> {
        raw_alloc::alloc_pages(0, AllocFlags::empty()).map(|p| p + virt::PHYSICAL_MAP_BASE)
    }
    unsafe fn dealloc_frame(&mut self, frame: VFrame<FrameSize4K>) -> Result<(), Self::Error> {
        raw_alloc::dealloc_pages(
            PFrame::new_unchecked(PAddr::new(
                (frame.addr() - virt::PHYSICAL_MAP_BASE.addr()).as_u64(),
            )),
            0,
        );
        Ok(())
    }
}

per_cpu! {
    static mut ref PAGE_TABLE: PageTable = PageTable::empty();
}

per_cpu_lazy! {
    static mut ref MAPPER: LoggingMapper<OffsetMapper<'static>> = unsafe { LoggingMapper::new(OffsetMapper::new(PAGE_TABLE.get_mut(), virt::PHYSICAL_MAP_BASE.addr())) };
}

pub unsafe fn init_kernel_virt() {
    PAGE_TABLE.with(|pgt| {
        copy_early_kernel_table_to(pgt);
        let vaddr = VAddr::from(pgt);
        let paddr = early_paddr_of(vaddr).expect("PerCpu should be mapped");
        PageTable::set_page_table(PFrame::new(paddr));
    });
}

pub fn map_frames<S: FrameSize>(
    range: PFrameRange<S>,
    vbase: VFrame<S>,
    flags: MapFlags,
) -> Result<(), AllocError>
where
    OffsetMapper<'static>: RangeMapper<S, PGTFrameSize = FrameSize4K>,
{
    unsafe {
        MAPPER.with_static(|mapper| {
            mapper
                .map_range(range, vbase, flags, &mut MMFrameAllocator)
                .map_err(|err| {
                    chos_lib::log::error!(
                        "Map error {:?}, tried to map {:#x}-{:#x} to {:#x} [{:?}]",
                        err,
                        range.start(),
                        range.end(),
                        vbase,
                        flags,
                    );
                    AllocError
                })?
                .flush();
            Ok(())
        })
    }
}

pub fn map_page(page: &Page, vbase: VFrame, flags: MapFlags) -> Result<(), AllocError> {
    map_frames(page.frame_range(), vbase, flags)
}
