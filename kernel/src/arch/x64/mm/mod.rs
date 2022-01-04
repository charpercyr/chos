use chos_lib::arch::mm::PAddr;
use chos_lib::boot::KernelBootInfo;
use chos_lib::log::debug;
use multiboot2::MemoryArea;

use crate::mm::phys::{add_regions, RegionFlags};

use self::virt::init_kernel_table;

mod per_cpu;
pub mod virt;

fn is_early_memory(area: &MemoryArea, info: &KernelBootInfo) -> bool {
    area.typ() == multiboot2::MemoryAreaType::Available
        && area.start_address() > info.mem_info.code.phys.as_u64() + info.mem_info.code.size as u64
        && area.start_address() > info.mem_info.pt.phys.as_u64() + info.mem_info.pt.size as u64
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
    init_kernel_table(info);
}
