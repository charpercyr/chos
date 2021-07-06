#![no_std]

#![feature(asm)]
#![feature(decl_macro)]
#![feature(thread_local)]
#![feature(never_type)]

mod arch;

use chos_boot_defs::KernelBootInfo;
use chos_x64::qemu::{exit_qemu, QemuStatus};

#[panic_handler]
fn panic(_: &core::panic::PanicInfo) -> ! {
    loop {}
}

macro_rules! rip {
    () => {
        unsafe {
            let rip: usize;
            asm! {
                "leaq (%rip), {}",
                lateout(reg) rip,
                options(att_syntax, nostack),
            };
            rip
        }
    };
}

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
    (info.early_log)(format_args!("[{}] Hello From the kernel @ {:016x} !", id, rip!()));
    static BARRIER: spin::Barrier = spin::Barrier::new(2);
    BARRIER.wait();
    if id == 0 {
        (info.early_log)(format_args!("Kernel mem info {:#x?}", info.mem_info));
        exit_qemu(QemuStatus::Success);
    } else {
        hlt_loop();
    }
}
chos_boot_defs::check_kernel_entry!(entry);
