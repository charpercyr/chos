use core::mem::MaybeUninit;

use chos_lib::arch::intr::enable_interrupts;
use chos_lib::arch::mm::VAddr;
use chos_lib::arch::port::PortWriteOnly;
use chos_lib::arch::regs::{Cr2, ScratchRegs};
use chos_lib::arch::tables::{interrupt, Handler, Idt, PageFaultError, StackFrame};
use chos_lib::arch::x64::acpi::madt::{self, Madt};
use chos_lib::arch::x64::apic::Apic;
use chos_lib::arch::x64::intr::without_interrupts;
use chos_lib::arch::x64::ioapic::{self, IOApic};
use rustc_demangle::demangle;

pub const INTERRUPT_SPURIOUS: u8 = 0xff;
pub const INTERRUPT_IOAPIC_BASE: u8 = 0x20;

#[interrupt]
extern "x86-interrupt" fn intr_breakpoint(f: &mut StackFrame<ScratchRegs>) {
    unsafe { crate::unsafe_println!("BREAKPOINT: {:#x?}", f) };
}

#[interrupt]
extern "x86-interrupt" fn intr_double_fault(f: &mut StackFrame<ScratchRegs>, _: u64) -> ! {
    panic!("DOUBLE FAULT: {:#x?}", f);
}

#[interrupt]
extern "x86-interrupt" fn intr_page_fault(f: &mut StackFrame<ScratchRegs>, e: PageFaultError) {
    use crate::unsafe_println;
    unsafe {
        if let Some((name, offset)) = super::symbols::find_symbol(f.intr.rip) {
            unsafe_println!(
                "PAGE FAULT @ 0x{:x} [{:#} + 0x{:x}]",
                f.intr.rip,
                demangle(name),
                offset,
            )
        } else {
            unsafe_println!("PAGE FAULT @ 0x{:x} [?]", f.intr.rip);
        }
        unsafe_println!("Tried to access 0x{:x} : {:?}", Cr2::read(), e);
    }
    panic!();
}

#[interrupt]
extern "x86-interrupt" fn intr_general_protection_fault(f: &mut StackFrame<ScratchRegs>, _: u64) {
    panic!("GPF: {:#x?}", f);
}

#[interrupt]
extern "x86-interrupt" fn intr_invalid_opcode(f: &mut StackFrame<ScratchRegs>) {
    panic!("Invalid Instruction: {:#x?}", f)
}

#[interrupt]
extern "x86-interrupt" fn intr_spurious(_: &mut StackFrame<ScratchRegs>) {
    // Nothing
}

fn disable_pic() {
    unsafe {
        PortWriteOnly::<u8>::new(0x21).write(0xff);
        PortWriteOnly::<u8>::new(0xa1).write(0xff);
    }
}

static mut IDT: Idt = Idt::empty();
static mut APIC: MaybeUninit<Apic> = MaybeUninit::uninit();
static mut IO_APIC: MaybeUninit<IOApic> = MaybeUninit::uninit();

pub fn initalize(madt: &Madt) {
    disable_pic();

    let idt = unsafe { &mut IDT };
    idt.breakpoint.set_handler(intr_breakpoint);
    idt.double_fault.set_handler(intr_double_fault);
    idt.page_fault.set_handler(intr_page_fault);
    idt.general_protection_fault
        .set_handler(intr_general_protection_fault);
    idt.invalid_opcode.set_handler(intr_invalid_opcode);

    let ioapic = madt
        .entries()
        .find_map(|e| {
            if let madt::Entry::IoApic(ioapic) = e {
                Some(ioapic)
            } else {
                None
            }
        })
        .expect("Expect at least 1 IOApic");

    let apic;
    unsafe {
        APIC = MaybeUninit::new(Apic::new(VAddr::new_unchecked(madt.lapic_address as u64)));
        IO_APIC = MaybeUninit::new(IOApic::new(VAddr::new(ioapic.ioapic_address as u64)));

        apic = APIC.assume_init_mut();
    }

    unsafe {
        idt[INTERRUPT_SPURIOUS as usize].set_handler(intr_spurious);
        apic.initialize();
    }

    unsafe { Idt::load(&IDT) };

    enable_interrupts();
}

pub unsafe fn apic() -> &'static mut Apic<'static> {
    APIC.assume_init_mut()
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct IOApicAllocFailed;

pub fn try_ioapic_alloc<R, F: FnOnce(&mut ioapic::RedirectionEntry) -> R>(
    n: u8,
    f: F,
    handler: Handler,
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
        idt[(INTERRUPT_IOAPIC_BASE + n) as usize].set_handler(handler);

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
