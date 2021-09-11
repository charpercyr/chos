
use super::regs::Flags;

pub struct IntrStatus(Flags);

pub fn disable_interrups() {
    unsafe {
        asm! {
            "cli",
            options(nomem, nostack),
        };
    }
}

pub fn enable_interrupts() {
    unsafe {
        asm! {
            "sti",
            options(nomem, nostack),
        }
    }
}

pub fn disable_interrups_save() -> IntrStatus {
    let flags = IntrStatus(Flags::get());
    disable_interrups();
    flags
}

pub fn restore_interrupts(status: IntrStatus) {
    unsafe {
        Flags::set(status.0);
    }
}

pub fn without_interrupts<R, F: FnOnce() -> R>(f: F) -> R {
    let flags = disable_interrups_save();
    let res = f();
    restore_interrupts(flags);
    res
}
