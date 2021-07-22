use core::panic::PanicInfo;

use rustc_demangle::demangle;

use chos_x64::backtrace;
use chos_x64::qemu::*;

use crate::unsafe_println;

static mut IN_PANIC: bool = false;
#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    unsafe {
        let in_panic = IN_PANIC;
        IN_PANIC = true;
        unsafe_println!("=== PANIC ===");
        unsafe_println!("{}", info);
        if !in_panic {
            unsafe_println!("BACKTRACE");
            for frame in backtrace().take(200) {
                if let Some((sym, offset)) = super::symbols::find_symbol(frame.as_u64() as usize) {
                    unsafe_println!("  {:016p} [{:#} + 0x{:x}]", frame, demangle(sym), offset);
                } else {
                    unsafe_println!("  {:016p} [?]", frame);
                }
            }
        }
        unsafe_println!();
    }
    exit_qemu(QemuStatus::Error);
}
