use core::panic::PanicInfo;

use chos_lib::arch::x64::qemu::exit_qemu;
use chos_lib::log::*;

#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    unsafe {
        unsafe_error!("========================");
        unsafe_error!("PANIC: {}", info);
        unsafe_info!("Backtrace");
        for frame in chos_lib::arch::x64::backtrace() {
            unsafe_info!("  0x{:016x}", frame.as_u64());
        }
        unsafe_error!("========================");
        exit_qemu(chos_lib::arch::x64::qemu::QemuStatus::Error)
    }
}
