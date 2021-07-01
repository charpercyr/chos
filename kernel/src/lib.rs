#![no_std]

#![feature(asm)]
#![feature(decl_macro)]
#![feature(thread_local)]

mod arch;

use chos_boot_defs::KernelBootInfo;

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

#[no_mangle]
pub fn entry(info: &KernelBootInfo, id: u8) -> ! {
    (info.early_log)(format_args!("Hello From the kernel [{}] @ {:016x} !", id, rip!()));
    loop {}
    // chos_x64::qemu::exit_qemu(chos_x64::qemu::QemuStatus::Success)
}
chos_boot_defs::check_kernel_entry!(entry);
