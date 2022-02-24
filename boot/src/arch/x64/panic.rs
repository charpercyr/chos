use core::panic::PanicInfo;

use chos_lib::arch::x64::backtrace;
use chos_lib::arch::x64::qemu::*;
use chos_lib::log::unsafe_error;

use chos_lib::log::unsafe_println;

static mut IN_PANIC: bool = false;
#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    unsafe {
        let in_panic = IN_PANIC;
        IN_PANIC = true;
        unsafe_error!("=== PANIC ===");
        unsafe_error!("{}", info);
        if !in_panic {
            unsafe_println!("BACKTRACE");
            for frame in backtrace().take(200) {
                unsafe_println!("  {:016p} [?]", frame);
            }
        }
        unsafe_println!();
    }
    exit_qemu(QemuStatus::Error);
}
