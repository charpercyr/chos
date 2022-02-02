mod bt;
pub use bt::*;
pub mod acpi;
pub mod apic;
pub mod boot;
pub mod cache;
pub mod hpet;
pub mod intr;
pub mod ioapic;
pub mod mm;
pub mod msr;
pub mod port;
pub mod qemu;
pub mod regs;
pub mod serial;
pub mod tables;

pub fn hlt_loop() -> ! {
    intr::disable_interrups();
    loop {
        unsafe {
            core::arch::asm! {
                "0: hlt",
                "jmp 0b",
                options(att_syntax, nomem, nostack, noreturn),
            }
        }
    }
}
