use core::sync::atomic::{AtomicUsize, Ordering};

use chos_config::arch::mm::{stack, virt};
use chos_lib::arch::acpi::madt;
use chos_lib::arch::apic::{self, Apic};
use chos_lib::arch::intr::{enable_interrupts, IoPl};
use chos_lib::arch::ioapic::{self, IOApic};
use chos_lib::arch::regs::{Cr2, Rsp};
use chos_lib::arch::tables::{interrupt, Descriptor, Gdt, Idt, PageFaultError, StackFrame, Tss};
use chos_lib::log::debug;
use chos_lib::mm::VAddr;
use chos_lib::sync::{SpinLazy, SpinOnceCell, Spinlock};

use crate::kmain::KernelArgs;
use crate::mm::virt::stack::alloc_kernel_stack;
use crate::mm::virt::{handle_kernel_page_fault, PageFaultReason, PageFaultResult};
use crate::mm::{per_cpu_lazy, PerCpu};

const TSS_SEGMENT: u16 = 0x18;

const PAGE_FAULT_IST: u8 = 1;
const DOUBLE_FAULT_IST: u8 = 2;

per_cpu_lazy! {
    static mut ref TSS: Tss = {
        let mut tss = Tss::new();

        let pf_stack = alloc_kernel_stack(stack::KERNEL_STACK_PAGE_ORDER).expect("Stack alloc should not fail");
        let df_stack = alloc_kernel_stack(stack::KERNEL_STACK_PAGE_ORDER).expect("Stack alloc should not fail");

        debug!("Using {:#x}-{:#x} for Page Fault Stack (ist = {})", pf_stack.range.start(), pf_stack.range.end(), PAGE_FAULT_IST);
        debug!("Using {:#x}-{:#x} for Double Fault Stack (ist = {})", df_stack.range.start(), df_stack.range.end(), DOUBLE_FAULT_IST);

        tss.ist[PAGE_FAULT_IST as usize] = pf_stack.range.end().addr();
        tss.ist[DOUBLE_FAULT_IST as usize] = pf_stack.range.end().addr();
        tss
    };
    static mut ref GDT: Gdt<6> = {
        let mut gdt = Gdt::new();
        let tss = unsafe { TSS.get_mut() };
        gdt[0].set_code64(IoPl::Ring0); // 0x08
        gdt[1].set_data64(IoPl::Ring0); // 0x10
        Descriptor::set_tss(&mut gdt[2..=3], tss); //0x18
        gdt[4].set_code64(IoPl::Ring3); //0x28
        gdt[5].set_data64(IoPl::Ring3); //0x30
        gdt
    };
}

static mut LAPIC: SpinOnceCell<Apic> = SpinOnceCell::new();
static IOAPIC: SpinOnceCell<Spinlock<IOApic>> = SpinOnceCell::new();

const IOAPIC_IDT_BASE: u8 = 0x20;
const IOAPIC_MAX_INTR: u8 = 24;
static IOAPIC_INTR_HANDLERS: [AtomicUsize; IOAPIC_MAX_INTR as usize] = {
    const INIT: AtomicUsize = AtomicUsize::new(0);
    [INIT; IOAPIC_MAX_INTR as usize]
};

macro_rules! ioapic_intr {
    ($($n:expr),* $(,)?) => {
        paste::item! {
            $(
                #[interrupt]
                extern "x86-interrupt" fn [<ioapic_intr_ $n>](frame: StackFrame) {
                    let handler = IOAPIC_INTR_HANDLERS[$n].load(Ordering::Relaxed);
                    if handler != 0 {
                        let handler: fn(StackFrame) = unsafe { core::mem::transmute(handler) };
                        handler(frame);
                    }
                    unsafe { LAPIC.as_mut_unchecked().eoi() };
                }
            )*
        }
    };
}
// Update if changing ioapic_max_intr
ioapic_intr!(0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20, 21, 22, 23);

fn is_addr_in_kernel(addr: VAddr) -> bool {
    addr >= virt::KERNEL_BASE.addr()
}

fn handle_intr_error(frame: StackFrame) {
    if is_addr_in_kernel(frame.intr.rip) {
        panic!("Error in kernel: {:#?}", frame);
    } else {
        todo!("Handle user errors");
    }
}

#[interrupt]
extern "x86-interrupt" fn intr_error(frame: StackFrame) {
    handle_intr_error(frame);
}

#[inline(always)]
fn rsp() -> u64 {
    unsafe {
        let rsp: u64;
        core::arch::asm! {
            "mov %rsp, {}",
            lateout(reg) rsp,
            options(att_syntax, nomem),
        }
        rsp
    }
}

#[interrupt]
extern "x86-interrupt" fn intr_double_fault(frame: StackFrame, _: u64) -> ! {
    panic!("DOUBLE FAULT: {:#x?}\nRSP = {:#x}", frame, rsp());
}

#[interrupt]
extern "x86-interrupt" fn intr_gpf(frame: StackFrame, _: u64) {
    handle_intr_error(frame);
}

#[interrupt]
extern "x86-interrupt" fn intr_breakpoint(frame: StackFrame) {
    debug!(
        "BREAKPOINT @ {:#x}, rsp = {:#x}",
        frame.intr.rip, frame.intr.rsp
    );
}

#[interrupt]
extern "x86-interrupt" fn intr_page_fault(frame: StackFrame, error: PageFaultError) {
    let vaddr = Cr2::read();
    let mut reason_str = "Not Mapped";
    if !error.contains(PageFaultError::USER_MODE | PageFaultError::PROTECTION_VIOLATION) {
        let reason = if error.contains(PageFaultError::CAUSED_BY_WRITE) {
            PageFaultReason::Write
        } else {
            PageFaultReason::Read
        };
        match handle_kernel_page_fault(vaddr, reason) {
            PageFaultResult::Mapped(_) => return,
            PageFaultResult::NotMapped => reason_str = "Not Mapped",
            PageFaultResult::StackOverflow => reason_str = "Stack Overflow",
        };
    }
    panic!(
        "PAGE FAULT because of: {}\n{:#x?} [{:?}]\nTried to access {:#x}\nRSP = {:#x}",
        reason_str,
        frame,
        error,
        Cr2::read(),
        Rsp::read(),
    );
}

static IDT: SpinLazy<Idt> = SpinLazy::new(|| {
    let mut idt = Idt::empty();

    // "Normal" errors
    idt.divide_error.set_handler(intr_error);
    idt.overflow.set_handler(intr_error);
    idt.bound_range_exceeded.set_handler(intr_error);
    idt.invalid_opcode.set_handler(intr_error);
    idt.x87_floating_point.set_handler(intr_error);
    idt.simd_floating_point.set_handler(intr_error);
    idt.general_protection_fault.set_handler(intr_gpf);

    // Breakpoint
    idt.breakpoint.set_handler(intr_breakpoint);

    // Page Fault
    idt.page_fault
        .set_handler(intr_page_fault)
        .set_stack_index(Some(PAGE_FAULT_IST));

    // Double Fault
    idt.double_fault
        .set_handler(intr_double_fault)
        .set_stack_index(Some(DOUBLE_FAULT_IST));

    // Update if changing ioapic_max_intr
    idt[(IOAPIC_IDT_BASE + 0) as usize].set_handler(ioapic_intr_0);
    idt[(IOAPIC_IDT_BASE + 1) as usize].set_handler(ioapic_intr_1);
    idt[(IOAPIC_IDT_BASE + 2) as usize].set_handler(ioapic_intr_2);
    idt[(IOAPIC_IDT_BASE + 3) as usize].set_handler(ioapic_intr_3);
    idt[(IOAPIC_IDT_BASE + 4) as usize].set_handler(ioapic_intr_4);
    idt[(IOAPIC_IDT_BASE + 5) as usize].set_handler(ioapic_intr_5);
    idt[(IOAPIC_IDT_BASE + 6) as usize].set_handler(ioapic_intr_6);
    idt[(IOAPIC_IDT_BASE + 7) as usize].set_handler(ioapic_intr_7);
    idt[(IOAPIC_IDT_BASE + 8) as usize].set_handler(ioapic_intr_8);
    idt[(IOAPIC_IDT_BASE + 9) as usize].set_handler(ioapic_intr_9);
    idt[(IOAPIC_IDT_BASE + 10) as usize].set_handler(ioapic_intr_10);
    idt[(IOAPIC_IDT_BASE + 11) as usize].set_handler(ioapic_intr_11);
    idt[(IOAPIC_IDT_BASE + 12) as usize].set_handler(ioapic_intr_12);
    idt[(IOAPIC_IDT_BASE + 13) as usize].set_handler(ioapic_intr_13);
    idt[(IOAPIC_IDT_BASE + 14) as usize].set_handler(ioapic_intr_14);
    idt[(IOAPIC_IDT_BASE + 15) as usize].set_handler(ioapic_intr_15);
    idt[(IOAPIC_IDT_BASE + 16) as usize].set_handler(ioapic_intr_16);
    idt[(IOAPIC_IDT_BASE + 17) as usize].set_handler(ioapic_intr_17);
    idt[(IOAPIC_IDT_BASE + 18) as usize].set_handler(ioapic_intr_18);
    idt[(IOAPIC_IDT_BASE + 19) as usize].set_handler(ioapic_intr_19);
    idt[(IOAPIC_IDT_BASE + 20) as usize].set_handler(ioapic_intr_20);
    idt[(IOAPIC_IDT_BASE + 21) as usize].set_handler(ioapic_intr_21);
    idt[(IOAPIC_IDT_BASE + 22) as usize].set_handler(ioapic_intr_22);
    idt[(IOAPIC_IDT_BASE + 23) as usize].set_handler(ioapic_intr_23);

    idt
});

pub unsafe fn arch_init_interrupts_cpu(args: &KernelArgs) {
    let rsdt = args.arch.rsdt();
    let madt = rsdt.madt().expect("MADT is missing");

    let lapic = LAPIC.as_mut_unchecked();

    lapic.initialize();

    let acpi_id = madt
        .lapics()
        .find(|&e| e.apic_id == lapic.id())
        .map(|e| e.acpi_processor_id)
        .unwrap_or_else(|| lapic.id());

    for nmi in madt
        .nmis()
        .filter(|&e| e.acpi_processor_id == 0xff || e.acpi_processor_id == acpi_id)
    {
        let lint = match nmi.lint {
            0 => lapic.lint0_mut(),
            1 => lapic.lint1_mut(),
            _ => unreachable!("Invalid MADT NMI Entry {:#?}", nmi),
        };
        lint.update(|lint| {
            debug!("Setting LINT{} as NMI [{:?}]", nmi.lint, nmi.flags);
            lint.set_pin_polarity(match nmi.flags.polarity() {
                madt::Polarity::ActiveLow => apic::Polarity::ActiveLow,
                madt::Polarity::ActiveHigh => apic::Polarity::ActiveHigh,
                madt::Polarity::Conforming => match nmi.flags.trigger_mode() {
                    madt::TriggerMode::Level => apic::Polarity::ActiveLow,
                    madt::TriggerMode::Edge => apic::Polarity::ActiveHigh,
                    madt::TriggerMode::Conforming => apic::Polarity::ActiveHigh,
                },
            });
            lint.set_trigger_mode(match nmi.flags.trigger_mode() {
                madt::TriggerMode::Conforming | madt::TriggerMode::Edge => apic::TriggerMode::Edge,
                madt::TriggerMode::Level => apic::TriggerMode::Level,
            });
            lint.set_delivery_mode(apic::DeliveryMode::Nmi);
            lint.set_mask(apic::InterruptMask::Enabled);
        });
    }

    Gdt::load(GDT.get_ref());
    Tss::load(TSS_SEGMENT);
    Idt::load(&IDT);
    enable_interrupts();
}

pub unsafe fn arch_init_interrupts(args: &KernelArgs) {
    let rsdt = args.arch.rsdt();
    let madt = rsdt.madt().expect("MADT is missing");
    let ioapic = madt
        .ioapics()
        .find(|&e| e.global_system_interrupt_base == 0)
        .expect("Should have at least 1 ioapic");
    let lapic_addr = virt::DEVICE_BASE.addr()
        + madt
            .lapic_address_override()
            .map(|a| a.address)
            .next()
            .unwrap_or(madt.lapic_address as u64);

    let mut ioapic = IOApic::new(virt::DEVICE_BASE.addr() + ioapic.ioapic_address as u64);

    for i in 0..ioapic.max_red_entries() {
        ioapic.update_redirection(i, |red| red.disable());
    }

    SpinOnceCell::force_set(&IOAPIC, Spinlock::new(ioapic))
        .ok()
        .expect("IOAPIC already init");
    SpinOnceCell::force_set(&LAPIC, Apic::new(lapic_addr))
        .ok()
        .expect("LAPIC already init");
}

#[derive(Debug, Clone, Copy)]
pub struct IoApicAllocateError;

pub fn allocate_ioapic_interrupt(
    mut mask: u64,
    intr_fn: fn(StackFrame),
    dest: ioapic::Destination,
) -> Result<u8, IoApicAllocateError> {
    let mut ioapic = IOAPIC.try_get().expect("IOApic not initialized").lock();
    mask &= (1 << u8::min(ioapic.max_red_entries(), IOAPIC_MAX_INTR)) - 1;
    while mask != 0 {
        let handler_idx = mask.trailing_zeros() as u8;
        unsafe {
            if ioapic.update_redirection(handler_idx, |red| {
                if red.enabled() {
                    false
                } else {
                    red.set_delivery_mode(ioapic::DeliveryMode::Fixed);
                    red.set_vector(IOAPIC_IDT_BASE + handler_idx);
                    red.set_destination(dest);
                    red.enable();
                    true
                }
            }) {
                IOAPIC_INTR_HANDLERS[handler_idx as usize]
                    .store(intr_fn as usize, Ordering::Relaxed);
                return Ok(handler_idx);
            };
        }
    }
    Err(IoApicAllocateError)
}

pub unsafe fn free_ioapic_interrupt(n: u8) {
    let mut ioapic = IOAPIC.try_get().expect("IOApic not initialized").lock();
    ioapic.update_redirection(n, |red| {
        red.disable();
    })
}
