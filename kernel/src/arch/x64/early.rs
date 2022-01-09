use chos_config::arch::mm::{phys, virt};
use chos_lib::arch::mm::{FrameSize1G, FrameSize4K, OffsetMapper, PAddr, PageTable, VAddr};
use chos_lib::boot::KernelBootInfo;
use chos_lib::elf::Elf;
use chos_lib::log::debug;
use chos_lib::mm::{
    LoggingMapper, MapFlags, MapperFlush, PFrame, PFrameRange, RangeMapper, VFrame,
};
use multiboot2::MemoryArea;

use super::mm::virt::MMFrameAllocator;
use crate::arch::mm::per_cpu::init_per_cpu_data;
use crate::mm::phys::{add_regions, RegionFlags};
use crate::mm::virt::{paddr_of, MemoryRegion};

static mut STACK_BASE: VFrame<FrameSize4K> = unsafe { VFrame::new_unchecked(virt::STACK_BASE) };

fn is_early_memory(area: &MemoryArea, info: &KernelBootInfo) -> bool {
    area.typ() == multiboot2::MemoryAreaType::Available
        && area.start_address() > info.mem_info.code.phys.as_u64() + info.mem_info.code.size as u64
}

unsafe fn setup_early_memory_allocator(info: &KernelBootInfo) {
    let mbh = multiboot2::load(info.arch.multiboot_header);
    if let Some(mem) = mbh.memory_map_tag() {
        let iter = mem.all_memory_areas().filter_map(|area| {
            is_early_memory(area, info).then(|| {
                debug!(
                    "Using {:#016x} - {:#016x} as early memory",
                    area.start_address(),
                    area.end_address()
                );
                (
                    PAddr::new(area.start_address()),
                    area.size(),
                    RegionFlags::empty(),
                )
            })
        });
        add_regions(iter);
    }
}

pub unsafe fn arch_init_early_memory(info: &KernelBootInfo) {
    setup_early_memory_allocator(info);
    init_early_kernel_table(info);
}

unsafe fn get_early_kernel_mapper() -> LoggingMapper<OffsetMapper<'static>> {
    static mut EARLY_KERNEL_TABLE: PageTable = PageTable::empty();
    LoggingMapper::new(OffsetMapper::new(
        &mut EARLY_KERNEL_TABLE,
        virt::PHYSICAL_MAP_BASE,
    ))
}

pub unsafe fn use_early_kernel_table() {
    let page_paddr = paddr_of(get_early_kernel_mapper().inner().p4.as_vaddr(), MemoryRegion::Static).unwrap();
    PageTable::set_page_table(PFrame::new_unchecked(page_paddr));
}

pub unsafe fn init_early_kernel_table(info: &KernelBootInfo) {
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
    let mut mapper = get_early_kernel_mapper();
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

    use_early_kernel_table();
    init_per_cpu_data(info.core_count, &elf, &mut mapper);
}

pub unsafe fn map_stack(pages: PAddr, count: u64, add_guard_page: bool) -> VAddr {
    let pages = PFrame::new_unchecked(pages);
    let mut mapper = get_early_kernel_mapper();
    let vbase = STACK_BASE;
    mapper
        .map_range(
            PFrameRange::new(pages, pages.add(count)),
            vbase,
            MapFlags::GLOBAL | MapFlags::WRITE,
            &mut MMFrameAllocator,
        )
        .unwrap()
        .ignore();
    STACK_BASE = STACK_BASE.add(count + add_guard_page.then_some(1).unwrap_or(0));
    vbase.addr()
}
