use alloc::vec::Vec;

use chos_config::arch::mm::{phys, virt};
use chos_lib::arch::boot::ArchKernelBootInfo;
use chos_lib::arch::mm::{FrameSize1G, FrameSize4K, OffsetMapper, PAddr, PageTable, VAddr};
use chos_lib::boot::{KernelBootInfo, KernelMemInfo};
use chos_lib::elf::Elf;
use chos_lib::log::debug;
use chos_lib::mm::{
    LoggingMapper, MapFlags, MapperFlush, PAddrResolver, PFrame, PFrameRange, RangeMapper, VFrame,
    VFrameRange,
};
use multiboot2::MemoryArea;

use super::kmain::ArchKernelArgs;
use super::mm::virt::MMFrameAllocator;
use crate::arch::mm::per_cpu::init_per_cpu_data;
use crate::kmain::KernelArgs;
use crate::mm::phys::{add_region, add_regions, RegionFlags};
use crate::mm::virt::{paddr_of, MemoryRegion};

fn is_early_memory(area: &MemoryArea, mem_info: &KernelMemInfo) -> bool {
    area.typ() == multiboot2::MemoryAreaType::Available
        && area.start_address() > mem_info.code.phys.as_u64() + mem_info.code.size as u64
}

fn is_non_early_memory(area: &MemoryArea, mem_info: &KernelMemInfo) -> bool {
    area.typ() == multiboot2::MemoryAreaType::Available
        && area.start_address() <= mem_info.code.phys.as_u64() + mem_info.code.size as u64
}

unsafe fn setup_early_memory_allocator(info: &KernelBootInfo) {
    let mbh = multiboot2::load(info.arch.multiboot_header as usize).expect("Could not load multiboot structure");
    if let Some(mem) = mbh.memory_map_tag() {
        let iter = mem.all_memory_areas().filter_map(|area| {
            is_early_memory(area, &info.mem_info).then(|| {
                debug!(
                    "Using {:#016x} - {:#016x} as early memory",
                    area.start_address(),
                    area.end_address()
                );
                (
                    PFrameRange::new(
                        PFrame::new_align_up(PAddr::new(area.start_address())),
                        PFrame::new_align_down(PAddr::new(area.start_address()) + area.size()),
                    ),
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

static mut EARLY_KERNEL_TABLE: PageTable = PageTable::empty();

unsafe fn get_early_kernel_mapper() -> LoggingMapper<OffsetMapper<'static>> {
    LoggingMapper::new(OffsetMapper::new(
        &mut EARLY_KERNEL_TABLE,
        virt::PHYSICAL_MAP_BASE,
    ))
}

pub unsafe fn use_early_kernel_table() {
    let page_paddr = paddr_of(
        get_early_kernel_mapper().inner().p4.as_vaddr(),
        MemoryRegion::Static,
    )
    .unwrap();
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

    let elf = Elf::new(info.elf.as_ref()).expect("Elf should be valid");
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
    static mut STACK_BASE: VFrame<FrameSize4K> = unsafe { VFrame::new_unchecked(virt::STACK_BASE) };
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

pub unsafe fn arch_copy_boot_data(data: &ArchKernelBootInfo) -> ArchKernelArgs {
    ArchKernelArgs {
        rsdt: data.rsdt,
        mbh: data.multiboot_header,
    }
}

pub unsafe fn copy_early_kernel_table_to(pgt: &mut PageTable) {
    for i in 256..512 {
        // core::ptr::write_volatile(&mut pgt[i], EARLY_KERNEL_TABLE[i]);
        pgt[i] = EARLY_KERNEL_TABLE[i];
    }
}

pub fn early_paddr_of(vaddr: VAddr) -> Option<PAddr> {
    unsafe { get_early_kernel_mapper().paddr_of(vaddr) }
}

pub unsafe fn unmap_early_lower_memory(mem_size: u64) {
    get_early_kernel_mapper()
        .unmap_range(
            VFrameRange::<FrameSize1G>::new(
                VFrame::null(),
                VFrame::new_align_up(VAddr::new(mem_size)),
            ),
            &mut MMFrameAllocator,
        )
        .unwrap()
        .ignore();
}

pub unsafe fn init_non_early_memory(args: &KernelArgs) {
    let mbh =
        multiboot2::load_with_offset(args.arch.mbh, virt::PHYSICAL_MAP_BASE.as_usize()).expect("Could not load multiboot structure");
    let mem_entries: Vec<_> = mbh
        .memory_map_tag()
        .expect("Should have a memory map")
        .all_memory_areas()
        .filter_map(|area| {
            is_non_early_memory(area, &args.mem_info).then(|| {
                (PAddr::new(area.start_address()), area.size())
            })
        })
        .collect();
    for (paddr, size) in mem_entries {
        if args.mem_info.code.phys >= paddr && args.mem_info.code.phys < paddr + size {
            let before = PFrameRange::new(
                PFrame::new_align_up(paddr),
                PFrame::new_align_down(args.mem_info.code.phys),
            );
            let after = PFrameRange::new(
                PFrame::new_align_up(args.mem_info.code.phys + args.mem_info.code.size as u64),
                PFrame::new_align_down(paddr + size),
            );
            debug!(
                "Using {:#016x}-{:#016x} as kernel memory",
                before.start(),
                before.end(),
            );
            debug!(
                "Using {:#016x}-{:#016x} as kernel memory",
                after.start(),
                after.end(),
            );
            if before.frame_count() > 0 {
                add_region(before, RegionFlags::empty());
            }
            if after.frame_count() > 0 {
                add_region(after, RegionFlags::empty());
            }
        } else {
            let area = PFrameRange::new(
                PFrame::new_align_up(paddr),
                PFrame::new_align_down(paddr + size),
            );
            debug!(
                "Using {:#016x}-{:#016x} as kernel memory",
                area.start(),
                area.end(),
            );
            add_region(
                area,
                RegionFlags::empty(),
            );
        }
    }
}
