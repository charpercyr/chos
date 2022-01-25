use chos_config::arch::mm::{stack, virt};
use chos_lib::arch::intr::{enable_interrupts, IoPl};
use chos_lib::arch::mm::VAddr;
use chos_lib::arch::regs::Cr2;
use chos_lib::arch::tables::{Descriptor, Gdt, Idt, InterruptStackFrame, PageFaultError, Tss};
use chos_lib::log::debug;
use chos_lib::sync::SpinLazy;

use crate::mm::{per_cpu, per_cpu_lazy, PerCpu};

const TSS_SEGMENT: u16 = 0x18;

per_cpu! {
    static mut ref DOUBLE_FAULT_STACK: [u8; stack::KERNEL_STACK_SIZE] = [0; stack::KERNEL_STACK_SIZE];
}

per_cpu_lazy! {
    static mut ref TSS: Tss = {
        let mut tss = Tss::new();
        // tss.ist[0] = VAddr::null(); // PAGE FAULT
        tss.ist[1] = DOUBLE_FAULT_STACK.with(|addr| VAddr::from(addr) + stack::KERNEL_STACK_SIZE as u64); // DOUBLE FAULT
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

extern "x86-interrupt" fn intr_double_fault(frame: InterruptStackFrame, _: u64) -> ! {
    unsafe {
        let rsp: u64;
        core::arch::asm! {
            "mov %rsp, {}",
            lateout(reg) rsp,
            options(att_syntax, nomem),
        }
        panic!("DOUBLE FAULT: {:#x?}, stack @ {:#x}", frame, rsp);
    }
}

extern "x86-interrupt" fn intr_gpf(frame: InterruptStackFrame, _: u64) {
    handle_intr_error(frame);
}

extern "x86-interrupt" fn intr_breakpoint(frame: InterruptStackFrame) {
    debug!("BREAKPOINT @ {:#x}, rsp = {:#x}", frame.ip, frame.sp);
}

extern "x86-interrupt" fn intr_page_fault(frame: InterruptStackFrame, error: PageFaultError) {
    panic!(
        "PAGE FAULT, TRYING TO ACCESS {:#016x} [{:?}]\n{:#?}",
        Cr2::read(),
        error,
        frame
    )
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
    idt.page_fault.set_handler(intr_page_fault);

    // Double Fault
    idt.double_fault
        .set_handler(intr_double_fault)
        .set_stack_index(Some(1));

    idt
});

pub unsafe fn arch_init_interrupts() {
    Gdt::load(GDT.get_ref());
    Tss::load(TSS_SEGMENT);
    Idt::load(&IDT);
    enable_interrupts();
}
