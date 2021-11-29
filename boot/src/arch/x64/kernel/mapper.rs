use chos_lib::arch::mm::{FrameSize1G, OffsetMapper};
use chos_lib::arch::x64::mm::{PAddr, VAddr};
use chos_lib::int::CeilDiv;
use chos_lib::log::debug;
use chos_lib::mm::{MapFlags, Mapper, MapperFlush, PFrame, VFrame};
use multiboot2::MemoryMapTag;

use super::palloc::PAlloc;

pub struct BootMapper {
    pub mapper: OffsetMapper<'static>,
}

impl BootMapper {
    pub unsafe fn new(alloc: &mut PAlloc) -> Self {
        let p4 = alloc.alloc_page_table();
        Self {
            mapper: OffsetMapper::identity(p4),
        }
    }

    pub unsafe fn identity_map_memory(
        &mut self,
        alloc: &mut PAlloc,
        map: &MemoryMapTag,
        base: VAddr,
    ) {
        const GB: u64 = 0x4000_0000;
        let mem_size = map
            .all_memory_areas()
            .map(|e| e.end_address())
            .max()
            .expect("Memory map is empty");
        let g_count = mem_size.ceil_div(GB);
        for i in 0..g_count {
            debug!(
                "IDENTITY MAP {:012x} -> {:016x}",
                i * GB,
                i * GB + base.as_u64()
            );
            self.mapper
                .map(
                    PFrame::<FrameSize1G>::new_unchecked(PAddr::new(i * GB)),
                    VFrame::new_unchecked(VAddr::new_unchecked(i * GB + base.as_u64())),
                    MapFlags::EXEC | MapFlags::WRITE,
                    alloc,
                )
                .unwrap()
                .ignore();
        }
    }

    pub unsafe fn set_page_table(&mut self) {
        self.mapper.p4.set_page_table()
    }
}
