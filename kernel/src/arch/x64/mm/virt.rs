use core::alloc::AllocError;

use chos_config::arch::mm::{phys, virt};
use chos_lib::arch::mm::{FrameSize1G, FrameSize4K, OffsetMapper, PAddr, PageTable, VAddr};
use chos_lib::boot::KernelBootInfo;
use chos_lib::elf::Elf;
use chos_lib::mm::{
    FrameAllocator, LoggingMapper, MapFlags, MapperFlush, PFrame, PFrameRange, RangeMapper, VFrame,
};

use crate::arch::mm::per_cpu::init_per_cpu_data;
use crate::mm::phys::{raw_alloc, AllocFlags};

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

static mut KERNEL_TABLE: PageTable = PageTable::empty();

pub unsafe fn init_kernel_table(info: &KernelBootInfo) {
    unsafe fn map(
        mapper: &mut impl RangeMapper<FrameSize1G, PGTFrameSize = FrameSize4K>,
        mem_size: u64,
        base: VFrame<FrameSize1G>,
        flags: MapFlags,
    ) {
        mapper
            .map_range(
                PFrameRange::<FrameSize1G>::new(
                    PFrame::null(),
                    PFrame::new_align_up(PAddr::new(mem_size)),
                ),
                base,
                flags,
                &mut MMFrameAllocator,
            )
            .expect("Map should succeed")
            .ignore();
    }
    let mut mapper = LoggingMapper::new(OffsetMapper::new(
        &mut KERNEL_TABLE,
        virt::PHYSICAL_MAP_BASE,
    ));
    let mem_size = info.mem_info.total_size;
    map(
        &mut mapper,
        mem_size,
        VFrame::null(),
        MapFlags::WRITE | MapFlags::EXEC,
    );
    map(
        &mut mapper,
        mem_size,
        VFrame::new_unchecked(virt::PHYSICAL_MAP_BASE),
        MapFlags::WRITE | MapFlags::GLOBAL,
    );
    map(
        &mut mapper,
        mem_size,
        VFrame::new_unchecked(virt::HEAP_BASE),
        MapFlags::WRITE | MapFlags::GLOBAL,
    );
    map(
        &mut mapper,
        mem_size,
        VFrame::new_unchecked(virt::DEVICE_BASE),
        MapFlags::WRITE | MapFlags::NOCACHE | MapFlags::GLOBAL,
    );

    let elf = Elf::new(&*info.elf).expect("Elf should be valid");
    mapper
        .map_elf_load_sections(
            &elf,
            PFrame::<FrameSize4K>::new_unchecked(phys::KERNEL_DATA_BASE),
            VFrame::new_unchecked(virt::STATIC_BASE),
            MapFlags::GLOBAL,
            &mut MMFrameAllocator,
        )
        .expect("Mapping failed")
        .ignore();
        

    let page_paddr = PAddr::new(
        (VAddr::from(&mut KERNEL_TABLE) - virt::STATIC_BASE + phys::KERNEL_DATA_BASE).as_u64(),
    );
    PageTable::set_page_table(PFrame::new_unchecked(page_paddr));

    init_per_cpu_data(info, &elf, &mut mapper);
}
