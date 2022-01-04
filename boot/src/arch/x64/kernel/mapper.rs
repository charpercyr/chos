use chos_lib::arch::mm::{FrameSize1G, OffsetMapper};
use chos_lib::arch::x64::mm::PAddr;
use chos_lib::mm::{
    LoggingMapper, MapFlags, MapperFlush, PFrame, PFrameRange, RangeMapper, VFrame,
};
use multiboot2::MemoryMapTag;

use super::palloc::PAlloc;

pub struct BootMapper {
    pub mapper: LoggingMapper<OffsetMapper<'static>>,
}

impl BootMapper {
    pub unsafe fn new(alloc: &mut PAlloc) -> Self {
        let p4 = alloc.alloc_page_table();
        Self {
            mapper: LoggingMapper::new(OffsetMapper::identity(p4)),
        }
    }

    pub unsafe fn identity_map_memory(
        &mut self,
        alloc: &mut PAlloc,
        map: &MemoryMapTag,
        vbase: VFrame<FrameSize1G>,
    ) {
        let mem_size = map
            .all_memory_areas()
            .map(|e| e.end_address())
            .max()
            .expect("Memory map is empty");
        let prange = PFrameRange::new(PFrame::null(), PFrame::new_align_up(PAddr::new(mem_size)));
        self.mapper
            .map_range(prange, vbase, MapFlags::EXEC | MapFlags::WRITE, alloc)
            .unwrap()
            .ignore();
    }
}
