
pub mod asm;
pub mod log;

use chos_x64::backtrace;

use core::panic::PanicInfo;

#[repr(u32)]
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
enum QemuStatus {
    Success = 0x10,
    Error = 0x11,
}

fn exit_qemu(status: QemuStatus) -> ! {
    unsafe {
        let status = status as u32;
        asm! {
            "outl %eax, $0xf4",
            in("eax") status,
            options(att_syntax),
        };
        core::hint::unreachable_unchecked()
    }
}

#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    unsafe {
        use crate::unsafe_println;
        unsafe_println!("=== PANIC ===");
        unsafe_println!("{}", info);
        unsafe_println!("BACKTRACE");
        for frame in backtrace() {
            unsafe_println!("  {:p}", frame);
        }
        unsafe_println!();
    }
    exit_qemu(QemuStatus::Error);
}

#[no_mangle]
pub extern "C" fn boot_main() -> ! {
    log::initialize(log::Device::Serial);
    
    #[cfg(test)]
    crate::test_main();

    exit_qemu(QemuStatus::Success);
}
