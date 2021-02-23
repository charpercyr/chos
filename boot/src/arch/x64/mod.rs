
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

#[no_mangle]
pub extern "C" fn boot_main(mbh: usize) -> ! {
    let mut logdev = None;

    let mbh = unsafe { mb::load(mbh) };

    if let Some(cmdline) = mbh.command_line_tag() {
        for kv in iter_cmdline(cmdline.command_line()) {
            match kv {
                ("output", Some("serial")) => logdev = Some(log::Device::Serial),
                ("output", Some("vga")) => logdev = Some(log::Device::Vga),
                _ => (),
            }
        }
    }
    let logdev = logdev.unwrap_or(log::Device::Serial);

    log::initialize(logdev);
    intr::initalize();
    
    if let Some(sections) = mbh.elf_sections_tag() {
        symbols::init_symbols(sections);
    }

    // unsafe {
    //     use x86_64::registers::model_specific::Msr;
    //     let apic_base = Msr::new(0x1b);
    //     let base_addr = apic_base.read();
    //     let id = core::ptr::read_volatile((base_addr + 0x020) as *const u32);
    //     println!("BASE: {:08x}", base_addr);
    //     println!("CPU ID {}", id);
    // }
    
    exit_qemu(QemuStatus::Success);
}
