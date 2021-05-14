mod acpi;
mod asm;
mod cmdline;
mod intr;
mod log;
mod kernel;
mod mpstart;
mod panic;
mod qemu;
mod symbols;
mod timer;

use crate::println;
use acpi::RSDT;
use cmdline::iter_cmdline;
use qemu::*;

use multiboot2 as mb;

#[no_mangle]
pub extern "C" fn boot_main(mbp: usize) -> ! {
    let mut logdev = log::Device::Serial;

    let mbh = unsafe { mb::load(mbp) };

    if let Some(cmdline) = mbh.command_line_tag() {
        for kv in iter_cmdline(cmdline.command_line()) {
            match kv {
                ("output", Some("serial")) => logdev = log::Device::Serial,
                ("output", Some("vga")) => logdev = log::Device::Vga,
                _ => (),
            }
        }
    }

    log::initialize(logdev);

    if let Some(sections) = mbh.elf_sections_tag() {
        symbols::init_symbols(sections);
    }

    let rsdt = mbh.rsdp_v1_tag().unwrap().rsdt_address();
    let rsdt = unsafe { &*(rsdt as *const RSDT) };
    let madt = rsdt.madt().unwrap();
    let hpet = rsdt.hpet().unwrap();

    intr::initalize(madt);

    let kernel = if let Some(kernel) = mbh.module_tags().find(|m| {
        if let Some(("kernel", _)) = iter_cmdline(m.name()).next() {
            true
        } else {
            false
        }
    }) {
        let kernel = unsafe { core::slice::from_raw_parts(
            kernel.start_address() as *const u8,
            (kernel.end_address() - kernel.start_address()) as usize)
        };
        chos_elf::Elf64::from_bytes(kernel).expect("Invalid kernel ELF")
    } else {
        panic!("No kernel")
    };

    let boot_memory_map = unsafe { kernel::map_kernel(&kernel) };

    timer::initialize(hpet);

    let count = core::sync::atomic::AtomicUsize::new(1);
    let n = unsafe { mpstart::start_mp(
        madt,
        |id, count| {
            let count: *const core::sync::atomic::AtomicUsize = count.cast();
            let count = &*count;
            println!("Hello from processor #{}", id);
            count.fetch_add(1, core::sync::atomic::Ordering::Release);
            loop {
                x86_64::instructions::hlt();
            }
        },
        &count as *const _ as _,
    ) };

    while count.load(core::sync::atomic::Ordering::Acquire) < n {
        core::hint::spin_loop();
    }

    exit_qemu(QemuStatus::Success);
}