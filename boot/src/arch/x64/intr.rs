use core::mem::MaybeUninit;

use chos_x64::apic::Apic;
use chos_x64::ioapic::{self, IOApic};
use rustc_demangle::demangle;
use x86_64::instructions::interrupts::without_interrupts;
use x86_64::instructions::port::PortWriteOnly;
use x86_64::registers::control::Cr2;
use x86_64::structures::idt::{
    HandlerFunc, InterruptDescriptorTable, InterruptStackFrame, PageFaultErrorCode,
};

use super::acpi::madt::{self, MADT};

pub const INTERRUPT_SPURIOUS: u8 = 0xff;
pub const INTERRUPT_IOAPIC_BASE: u8 = 0x20;

extern "x86-interrupt" fn intr_breakpoint(f: InterruptStackFrame) {
    unsafe { crate::unsafe_println!("BREAKPOINT: {:?}", f) };
}

extern "x86-interrupt" fn intr_double_fault(f: InterruptStackFrame, _: u64) -> ! {
    panic!("DOUBLE FAULT: {:?}", f);
}

extern "x86-interrupt" fn intr_page_fault(f: InterruptStackFrame, e: PageFaultErrorCode) {
    use crate::unsafe_println;
    unsafe {
        if let Some((name, offset)) =
            super::symbols::find_symbol(f.instruction_pointer.as_u64() as _)
        {
            unsafe_println!(
                "PAGE FAULT @ 0x{:x} [{:#} + 0x{:x}]",
                f.instruction_pointer.as_u64(),
                demangle(name),
                offset,
            )
        } else {
            unsafe_println!("PAGE FAULT @ 0x{:x} [?]", f.instruction_pointer.as_u64());
        }
        unsafe_println!("Tried to access 0x{:x} : {:?}", Cr2::read().as_u64(), e);
    }
    panic!();
}

extern "x86-interrupt" fn intr_spurious(_: InterruptStackFrame) {
    // Nothing
}

fn disable_pic() {
    unsafe {
        PortWriteOnly::<u8>::new(0x21).write(0xff);
        PortWriteOnly::<u8>::new(0xa1).write(0xff);
    }
}

static mut IDT: InterruptDescriptorTable = InterruptDescriptorTable::new();
static mut APIC: MaybeUninit<Apic> = MaybeUninit::uninit();
static mut IO_APIC: MaybeUninit<IOApic> = MaybeUninit::uninit();

pub fn initalize(madt: &MADT) {
    disable_pic();

    let idt = unsafe { &mut IDT };
    idt.breakpoint.set_handler_fn(intr_breakpoint);
    idt.double_fault.set_handler_fn(intr_double_fault);
    idt.page_fault.set_handler_fn(intr_page_fault);

    let ioapic = madt
        .entries()
        .find_map(|e| {
            if let madt::Entry::IOAPIC(ioapic) = e {
                Some(ioapic)
            } else {
                None
            }
        })
        .expect("Expect at least 1 IOApic");

    let apic;
    unsafe {
        APIC = MaybeUninit::new(Apic::with_address(madt.lapic_address as usize));
        IO_APIC = MaybeUninit::new(IOApic::with_address(ioapic.ioapic_address as usize));

        apic = APIC.assume_init_mut();
    }

    unsafe {
        IDT[INTERRUPT_SPURIOUS as usize].set_handler_fn(intr_spurious);
        apic.initialize(INTERRUPT_SPURIOUS);
    }

    idt.load();

    x86_64::instructions::interrupts::enable();
}

pub unsafe fn apic() -> &'static mut Apic {
    APIC.assume_init_mut()
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct IOApicAllocFailed;

pub fn try_ioapic_alloc<R, F: FnOnce(&mut ioapic::RedirectionEntry) -> R>(
    n: u8,
    f: F,
    handler: HandlerFunc,
) -> Result<R, IOApicAllocFailed> {
    without_interrupts(move || {
        let ioapic = unsafe { IO_APIC.assume_init_mut() };
        if n >= ioapic.max_red_entries() {
            return Err(IOApicAllocFailed);
        }
        let mut red = ioapic.read_redirection(n);
        if red.enabled() {
            return Err(IOApicAllocFailed);
        }

        let idt = unsafe { &mut IDT };
        idt[(INTERRUPT_IOAPIC_BASE + n) as usize].set_handler_fn(handler);

        let res = f(&mut red);
        red.enable();
        red.set_vector(INTERRUPT_IOAPIC_BASE + n);
        unsafe { ioapic.write_redirection(n, red) };

        Ok(res)
    })
}

pub unsafe fn eoi() {
    let apic = apic();
    apic.eoi();
}
