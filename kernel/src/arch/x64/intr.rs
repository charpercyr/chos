use core::sync::atomic::{AtomicUsize, Ordering};

use chos_config::arch::mm::virt;
use chos_lib::arch::acpi::madt;
use chos_lib::arch::apic::{self, Apic};
use chos_lib::arch::intr::{enable_interrupts, IoPl};
use chos_lib::arch::ioapic::{self, IOApic};
use chos_lib::arch::mm::VAddr;
use chos_lib::arch::regs::Cr2;
use chos_lib::arch::tables::{Descriptor, Gdt, Idt, InterruptStackFrame, PageFaultError, Tss};
use chos_lib::log::{debug, println};
use chos_lib::sync::{SpinLazy, SpinOnceCell, Spinlock};

use crate::kmain::KernelArgs;
use crate::mm::stack::{allocate_kernel_stacks, Stacks};
use crate::mm::{per_cpu_lazy, this_cpu_info, PerCpu};

const TSS_SEGMENT: u16 = 0x18;

const PAGE_FAULT_IST: u8 = 0;
static PAGE_FAULT_STACK: SpinOnceCell<Stacks> = SpinOnceCell::new();

const DOUBLE_FAULT_IST: u8 = 1;
static DOUBLE_FAULT_STACK: SpinOnceCell<Stacks> = SpinOnceCell::new();

per_cpu_lazy! {
    static mut ref TSS: Tss = {
        let mut tss = Tss::new();

        let cpu_id = this_cpu_info().id;
        let (pf_base, pf_size) = PAGE_FAULT_STACK.try_get().expect("PAGE_FAULT_STACK should be init").get_for(cpu_id);
        let (df_base, df_size) = DOUBLE_FAULT_STACK.try_get().expect("DOUBLE_FAULT_STACK should be init").get_for(cpu_id);

        debug!("Using {:#x} for Page Fault Stack (ist = 0)", pf_base + pf_size);
        debug!("Using {:#x} for Double Fault Stack (ist = 1)", df_base + df_size);

        tss.ist[PAGE_FAULT_IST as usize] = pf_base + pf_size;
        tss.ist[DOUBLE_FAULT_IST as usize] = df_base + df_size;
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
                extern "x86-interrupt" fn [<ioapic_intr_ $n>](frame: InterruptStackFrame) {
                    let handler = IOAPIC_INTR_HANDLERS[$n].load(Ordering::Relaxed);
                    if handler != 0 {
                        let handler: fn(InterruptStackFrame) = unsafe { core::mem::transmute(handler) };
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
    addr >= virt::KERNEL_BASE
}

fn handle_intr_error(frame: InterruptStackFrame) {
    if is_addr_in_kernel(frame.ip) {
        panic!("Error in kernel: {:#?}", frame);
    } else {
        todo!("Handle user errors");
    }
}

extern "x86-interrupt" fn intr_error(frame: InterruptStackFrame) {
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

extern "x86-interrupt" fn intr_double_fault(frame: InterruptStackFrame, _: u64) -> ! {
    panic!("DOUBLE FAULT: {:#x?}\nRSP = {:#x}", frame, rsp());
}

extern "x86-interrupt" fn intr_gpf(frame: InterruptStackFrame, _: u64) {
    handle_intr_error(frame);
}

extern "x86-interrupt" fn intr_breakpoint(frame: InterruptStackFrame) {
    debug!("BREAKPOINT @ {:#x}, rsp = {:#x}", frame.ip, frame.sp);
}

extern "x86-interrupt" fn intr_page_fault(frame: InterruptStackFrame, error: PageFaultError) {
    panic!(
        "PAGE FAULT: {:#x?} [{:?}]\nTried to access {:#x}\nRSP = {:#x}",
        frame,
        error,
        Cr2::read(),
        rsp()
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

    idt[0x80].set_handler({
        extern "x86-interrupt" fn callback(_: InterruptStackFrame) {
            println!("Hello");
            unsafe { LAPIC.as_mut_unchecked().eoi() };
        }
        callback
    });

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
                madt::Polarity::Conforming
                    if nmi.flags.trigger_mode() == madt::TriggerMode::Level =>
                {
                    apic::Polarity::ActiveLow
                }
                madt::Polarity::Conforming => apic::Polarity::ActiveHigh,
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
    let lapic_addr = virt::DEVICE_BASE
        + madt
            .lapic_address_override()
            .map(|a| a.address)
            .next()
            .unwrap_or(madt.lapic_address as u64);

    let mut ioapic = IOApic::new(virt::DEVICE_BASE + ioapic.ioapic_address as u64);

    for i in 0..ioapic.max_red_entries() {
        ioapic.update_redirection(i, |red| red.disable());
    }

    SpinOnceCell::force_set(&PAGE_FAULT_STACK, allocate_kernel_stacks(args.core_count))
        .ok()
        .expect("PAGE_FAULT_STACK already init");
    SpinOnceCell::force_set(&DOUBLE_FAULT_STACK, allocate_kernel_stacks(args.core_count))
        .ok()
        .expect("DOUBLE_FAULT_STACK already init");
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
    intr_fn: fn(InterruptStackFrame),
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
