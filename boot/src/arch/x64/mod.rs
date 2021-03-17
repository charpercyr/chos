mod acpi;
mod asm;
mod cmdline;
mod intr;
mod log;
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

use core::slice;

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

    intr::initalize();
    timer::initialize();

    // timer::delay(core::time::Duration::from_micros(100));

    let rsdt = mbh.rsdp_v1_tag().unwrap().rsdt_address();
    let rsdt = unsafe { &*(rsdt as *const RSDT) };
    let madt = rsdt.madt().unwrap();

    unsafe { mpstart::start_mp(
        madt,
        |id, _| {
            println!("Hello from processor #{}", id);
            loop {}
        },
        core::ptr::null(),
    ) };
    
    exit_qemu(QemuStatus::Success);
}
