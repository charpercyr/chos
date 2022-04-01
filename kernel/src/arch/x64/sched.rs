use core::arch::asm;
use core::mem::{size_of, MaybeUninit};
use core::sync::atomic::{AtomicU64, Ordering};

use chos_lib::arch::intr::IoPl;
use chos_lib::arch::regs::{Flags, IntrRegs, ScratchRegs, CS};
use chos_lib::mm::VAddr;

use crate::arch::intr::KERNEL_CS;
use crate::mm::virt::stack::Stack;
use crate::sched::TaskArc;

const REG_DEFAULT_VALUE: u64 = 0xcafebeefdeadbabe;

unsafe fn enter_first_task(stack: VAddr) -> ! {
    asm!(
        "mov {stack}, %rsp",
        "pop %rax",
        "pop %r11",
        "pop %r10",
        "pop %r9",
        "pop %r8",
        "pop %rcx",
        "pop %rdx",
        "pop %rsi",
        "pop %rdi",
        "add $8, %rsp",
        "iretq",
        stack = in(reg) stack.as_u64(),
        options(att_syntax, noreturn),
    )
}

#[naked]
unsafe extern "C" fn switch_task(new_stack: VAddr, old_state: *mut ArchTaskState) {
    // RDI = new_stack
    // RSI = old_stack
    asm!(
        "mov %rsp, %rax",
        "pushq $0",  // SS
        "push %rax", // RSP
        "pushf",     // RFlags
        "pushq ${KERNEL_CS}",
        "leaq 0f(%rip), %rax",
        "push %rax", // RIP
        "pushq $0",  // Error
        "push %rdi",
        "push %rsi",
        "push %rdx",
        "push %rcx",
        "push %r8",
        "push %r9",
        "push %r10",
        "push %r11",
        "push %rax",
        "mov %rsp, (%rsi)",
        "movq $1, 8(%rsi)",
        "mov %rdi, %rsp",
        "pop %rax",
        "pop %r11",
        "pop %r10",
        "pop %r9",
        "pop %r8",
        "pop %rcx",
        "pop %rdx",
        "pop %rsi",
        "pop %rdi",
        "add $8, %rsp", // Error
        "iretq",
        "0:",
        "ret",
        KERNEL_CS = const KERNEL_CS as u64,
        options(att_syntax, noreturn),
    )
}

#[naked]
unsafe extern "C" fn setup_task() -> ! {
    asm!(
        "push $0",
        "mov $0, %rbp",
        "call *%rax",
        "ud2",
        options(att_syntax, noreturn),
    )
}

#[repr(C)]
pub struct ArchTaskState {
    rsp: VAddr,
    sched_lock: AtomicU64,
}

impl ArchTaskState {
    pub fn with_fn(stack: Stack, fun: fn() -> !) -> Self {
        unsafe {
            let base = stack.range.end().addr();
            let regs_addr = base - size_of::<ScratchRegs>() as u64;
            let regs: &mut MaybeUninit<ScratchRegs> = regs_addr.as_mut();
            *regs = MaybeUninit::new(ScratchRegs {
                rax: fun as u64,
                r11: REG_DEFAULT_VALUE,
                r10: REG_DEFAULT_VALUE,
                r9: REG_DEFAULT_VALUE,
                r8: REG_DEFAULT_VALUE,
                rcx: REG_DEFAULT_VALUE,
                rdx: REG_DEFAULT_VALUE,
                rsi: REG_DEFAULT_VALUE,
                rdi: REG_DEFAULT_VALUE,
                intr: IntrRegs {
                    error: 0,
                    rip: VAddr::new_unchecked(setup_task as u64),
                    cs: CS::read() as u64,
                    rflags: Flags::new().with_intr_enable(true).with_iopl(IoPl::Ring0),
                    rsp: base,
                    ss: 0,
                },
            });
            Self {
                rsp: regs_addr,
                sched_lock: AtomicU64::new(1),
            }
        }
    }

    pub fn enter_first_task(task: TaskArc) -> ! {
        unsafe {
            enter_first_task({
                let state = task.state.lock();
                state.arch.rsp
            })
        }
    }

    pub fn switch_to(old: TaskArc, new: TaskArc) {
        assert_ne!(
            old.get_ptr(),
            new.get_ptr(),
            "Cannot switch to the same task"
        );
        let new_stack = {
            let new_state = new.state.lock_nodisable();
            new_state.arch.rsp
        };
        let old_stack_ptr = {
            let mut old_state = old.state.lock_nodisable();
            assert!(old_state
                .arch
                .sched_lock
                .compare_exchange(1, 0, Ordering::Acquire, Ordering::Relaxed)
                .is_ok());
            (&mut old_state.arch) as *mut ArchTaskState
        };
        unsafe { switch_task(new_stack, old_stack_ptr) }
    }
}
