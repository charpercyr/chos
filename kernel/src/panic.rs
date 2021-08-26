use core::fmt::Arguments;
use core::panic::PanicInfo;

use chos_x64::qemu::exit_qemu;

pub type PanicLogger = fn(Arguments);
static mut PANIC_LOGGER: Option<PanicLogger> = None;

#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    unsafe {
        if let Some(logger) = PANIC_LOGGER {
            logger(format_args!("PANIC: {}", info));
            logger(format_args!("Backtrace"));
            for frame in chos_x64::backtrace() {
                logger(format_args!("  0x{:016x}", frame.as_u64()));
            }
        }
        exit_qemu(chos_x64::qemu::QemuStatus::Error)
    }
}

pub unsafe fn set_panic_logger(logger: PanicLogger) {
    PANIC_LOGGER = Some(logger);
}
