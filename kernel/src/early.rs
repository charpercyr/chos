use core::arch::asm;

use chos_lib::arch::x64::qemu::{exit_qemu, QemuStatus};
use chos_lib::boot::KernelBootInfo;
use chos_lib::check_kernel_entry;
use chos_lib::log::*;

use crate::mm::init_early_memory;

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

#[no_mangle]
pub fn entry(info: &KernelBootInfo, id: u8) -> ! {
    if id != 0 {
        hlt_loop();
    }
    unsafe { chos_lib::log::set_handler(info.early_log) };

    debug!("####################");
    debug!("### EARLY KERNEL ###");
    debug!("####################");

    unsafe {
        init_early_memory(info);
    }

    exit_qemu(QemuStatus::Success);
}
check_kernel_entry!(entry);
