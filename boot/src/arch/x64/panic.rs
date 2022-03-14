use core::panic::PanicInfo;

use chos_lib::arch::x64::qemu::*;
use chos_lib::log::unsafe_error;

static mut IN_PANIC: bool = false;
#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    unsafe {
        let in_panic = IN_PANIC;
        IN_PANIC = true;
        unsafe_error!("=== PANIC ===");
        unsafe_error!("{}", info);
    }
    exit_qemu(QemuStatus::Error);
}
