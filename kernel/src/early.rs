use chos_config::arch::mm::virt;
use chos_lib::arch::x64::qemu::{exit_qemu, QemuStatus};
use chos_lib::boot::KernelBootInfo;
use chos_lib::check_kernel_entry;
use chos_lib::log::*;
use multiboot2::MemoryArea;

use super::*;

fn hlt_loop() -> ! {
    unsafe {
        asm! {
            "cli",
            "0: hlt",
            "jmp 0b",
            options(nomem, nostack, att_syntax, noreturn),
        }
    }
}

fn is_early_memory(area: &MemoryArea, info: &KernelBootInfo) -> bool {
    area.typ() == multiboot2::MemoryAreaType::Available
        && area.start_address() > info.mem_info.code.phys.as_u64() + info.mem_info.code.size as u64
        && area.start_address() > info.mem_info.pt.phys.as_u64() + info.mem_info.pt.size as u64
}

fn setup_early_memory_allocator(info: &KernelBootInfo) {
    let mbh = unsafe { multiboot2::load(info.multiboot_header) };
    if let Some(mem) = mbh.memory_map_tag() {
        let iter = mem.all_memory_areas().filter_map(|area| {
            is_early_memory(area, info).then(|| {
                debug!(
                    "Using {:#016x} - {:#016x} as early memory",
                    area.start_address(),
                    area.end_address()
                );
                (
                    chos_lib::arch::mm::PAddr::new(area.start_address()),
                    area.size(),
                    mm::phys::RegionFlags::empty(),
                )
            })
        });
        unsafe { mm::phys::add_regions(iter) };
    }
}

#[no_mangle]
pub fn entry(info: &KernelBootInfo, id: u8) -> ! {
    if id != 0 {
        hlt_loop();
    }
    unsafe { chos_lib::log::set_handler(info.early_log) };

    info!("####################");
    info!("### EARLY KERNEL ###");
    info!("####################");

    debug!("PHYSICAL_MAP_BASE   {:?}", virt::PHYSICAL_MAP_BASE);
    debug!("PAGING_BASE         {:?}", virt::PAGING_BASE);
    debug!("DEVICE_BASE         {:?}", virt::DEVICE_BASE);
    debug!("STATIC_BASE         {:?}", virt::STATIC_BASE);
    debug!("HEAP_BASE           {:?}", virt::HEAP_BASE);
    debug!("PERCPU_STATIC_BASE  {:?}", virt::PERCPU_STATIC_BASE);
    debug!("PERCPU_HEAP_BASE    {:?}", virt::PERCPU_HEAP_BASE);
    debug!("STACK_BASE          {:?}", virt::STACK_BASE);

    setup_early_memory_allocator(info);

    unsafe {
        const ORDER: u8 = 0;
        use mm::phys::{alloc_pages, AllocFlags};
        for _ in 0..10 {
            match alloc_pages(ORDER, AllocFlags::empty()) {
                Ok(page) => {
                    info!("ALLOC {:?}", page);
                    // core::mem::forget(page);
                }
                Err(e) => {
                    error!("ALLOC FAILED: {:?}", e);
                }
            }
        }
    }

    exit_qemu(QemuStatus::Success);
}
check_kernel_entry!(entry);
