mod acpi;
mod asm;
mod cmdline;
mod intr;
mod log;
mod panic;
mod qemu;
mod symbols;

use crate::println;
use cmdline::iter_cmdline;
use qemu::*;

use multiboot2 as mb;

// use chos_x64::apic::Apic;

use core::slice;

#[no_mangle]
pub extern "C" fn boot_main(mbh: usize) -> ! {
    let mut logdev = log::Device::Serial;

    let mbh = unsafe { mb::load(mbh) };

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

    let mut kernel = None;
    for module in mbh.module_tags() {
        match module.name() {
            "kernel" => kernel = Some(module),
            _ => (),
        };
    }

    if let Some(kernel) = kernel {
        let elf = unsafe {
            chos_elf::Elf64::from_bytes_unchecked(slice::from_raw_parts(
                kernel.start_address() as usize as *const u8,
                (kernel.end_address() - kernel.start_address()) as usize,
            ))
        };
        for prog in elf.program() {
            println!("{:?}", prog);
        }
    }
    exit_qemu(QemuStatus::Success);
}
