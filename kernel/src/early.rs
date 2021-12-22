use chos_lib::arch::x64::qemu::{exit_qemu, QemuStatus};
use chos_lib::boot::KernelBootInfo;
use chos_lib::check_kernel_entry;
use chos_lib::log::*;
use multiboot2::MemoryArea;

use crate::mm::phys::{alloc_pages, AllocFlags};

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

    debug!("####################");
    debug!("### EARLY KERNEL ###");
    debug!("####################");

    setup_early_memory_allocator(info);

    unsafe {
        let page = alloc_pages(10, AllocFlags::empty()).unwrap();
        println!("{:?}", page);
    }

    unsafe {
        let rsdt = &*info.arch.rsdt;
        let mcfg = rsdt.mcfg().unwrap();
        for seg in mcfg {
            println!("{:#x?}", seg);
        }
    }

    exit_qemu(QemuStatus::Success);
}
check_kernel_entry!(entry);
