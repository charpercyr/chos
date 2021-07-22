#![no_std]
#![feature(allocator_api)]
#![feature(asm)]
#![feature(const_fn_trait_bound)]
#![feature(const_fn_transmute)]
#![feature(default_alloc_error_handler)]
#![feature(decl_macro)]
#![feature(maybe_uninit_slice)]
#![feature(never_type)]
#![feature(option_result_unwrap_unchecked)]
#![feature(thread_local)]

extern crate alloc;

mod arch;
mod log;
mod mm;
mod panic;

use chos_boot_defs::KernelBootInfo;
use chos_x64::qemu::{exit_qemu, QemuStatus};
use multiboot2::MemoryArea;
use panic::set_panic_logger;

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
                    arch::mm::PAddr::new(area.start_address()),
                    area.size() / arch::mm::PAGE_SIZE64 * arch::mm::PAGE_SIZE64,
                    arch::mm::VAddr::make_canonical(area.start_address()),
                )
            })
        });
        unsafe { mm::phys::alloc::add_regions(iter) };
    }
}

#[no_mangle]
pub fn entry(info: &KernelBootInfo, id: u8) -> ! {
    if id != 0 {
        hlt_loop();
    }
    unsafe { set_panic_logger(info.early_log) };
    log::use_early_debug(info.early_log);
    setup_early_memory_allocator(info);

    let _ = alloc::boxed::Box::new(0);

    exit_qemu(QemuStatus::Success);
}
chos_boot_defs::check_kernel_entry!(entry);
