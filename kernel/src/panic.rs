use core::panic::PanicInfo;
use core::sync::atomic::{AtomicBool, Ordering};

use chos_lib::arch::x64::qemu::exit_qemu;
use chos_lib::log::*;
use rustc_demangle::demangle;

use crate::symbols::lookup_symbol;

static IN_PANIC: AtomicBool = AtomicBool::new(false);

pub fn in_panic() -> bool {
    IN_PANIC.load(Ordering::Relaxed)
}

#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    if IN_PANIC
        .compare_exchange(false, true, Ordering::Relaxed, Ordering::Relaxed)
        .is_ok()
    {
        unsafe {
            unsafe_error!("========================");
            unsafe_error!("PANIC: {}", info);
            unsafe_info!("Backtrace");
            for frame in chos_lib::arch::x64::backtrace() {
                if lookup_symbol(frame, |name, _, off| {
                    unsafe_info!("  {:#016x} [{:#} + {:#x}]", frame, demangle(name), off);
                })
                .is_none()
                {
                    unsafe_info!("  {:#016x} [?]", frame.as_u64());
                }
            }
            unsafe_error!("========================");
        }
    }
    exit_qemu(chos_lib::arch::x64::qemu::QemuStatus::Error)
}
