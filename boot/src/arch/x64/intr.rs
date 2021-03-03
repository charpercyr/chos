
use rustc_demangle::demangle;

use spin::Lazy;

use x86_64::instructions::port::PortWriteOnly;
use x86_64::registers::control::Cr2;
use x86_64::structures::idt::{InterruptDescriptorTable, InterruptStackFrame, PageFaultErrorCode};

extern "x86-interrupt" fn intr_breakpoint(f: &mut InterruptStackFrame) {
    unsafe { crate::unsafe_println!("BREAKPOINT: {:?}", f) };
}

extern "x86-interrupt" fn intr_double_fault(f: &mut InterruptStackFrame, _: u64) -> ! {
    panic!("DOUBLE FAULT: {:?}", f);
}

extern "x86-interrupt" fn intr_page_fault(f: &mut InterruptStackFrame, _: PageFaultErrorCode) {
    use crate::unsafe_println;
    unsafe {
        if let Some((name, offset)) = super::symbols::find_symbol(f.instruction_pointer.as_u64() as _) {
            unsafe_println!("PAGE FAULT @ 0x{:x} [{:#} + 0x{:x}]", f.instruction_pointer.as_u64(), demangle(name), offset)
        } else {
            unsafe_println!("PAGE FAULT @ 0x{:x} [?]", f.instruction_pointer.as_u64());
        }
        unsafe_println!("Tried to access 0x{:x}", Cr2::read().as_u64());
    }
    panic!();
}

extern "x86-interrupt" fn intr_timer(_: &mut InterruptStackFrame) {
    unsafe { crate::unsafe_print!(".") };
}

extern "x86-interrupt" fn intr_spurious(_: &mut InterruptStackFrame) {
    // Nothing
}

fn disable_pic() {
    unsafe {
        PortWriteOnly::<u8>::new(0x21).write(0xff);
        PortWriteOnly::<u8>::new(0xa1).write(0xff);
    }
}

static IDT: Lazy<InterruptDescriptorTable> = Lazy::new(|| {
    disable_pic();
    let mut idt = InterruptDescriptorTable::new();

    idt.breakpoint.set_handler_fn(intr_breakpoint);
    idt.double_fault.set_handler_fn(intr_double_fault);
    idt.page_fault.set_handler_fn(intr_page_fault);

    idt[0x20].set_handler_fn(intr_timer);
    idt[0xff].set_handler_fn(intr_spurious);

    idt
});

pub fn initalize() {
    Lazy::force(&IDT).load();
    x86_64::instructions::interrupts::enable();
}
