use core::arch::asm;
use core::mem::{size_of, MaybeUninit};

use chos_lib::arch::intr::IoPl;
use chos_lib::arch::regs::{Flags, IntrRegs, ScratchRegs, CS};
use chos_lib::mm::VAddr;

use crate::mm::virt::stack::Stack;
use crate::sched::Task;

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
unsafe extern "C" fn setup_task() -> ! {
    asm!(
        "push $0",
        "mov $0, %rbp",
        "call *%rax",
        "ud2",
        options(att_syntax, noreturn),
    )
}

pub struct ArchTask {
    pub rsp: VAddr,
    pub fsbase: u64,
    pub gsbase: u64,
}

impl ArchTask {
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
                fsbase: 0,
                gsbase: 0,
            }
        }
    }

    pub fn enter_first_task(task: &Task) -> ! {
        unsafe { enter_first_task(task.arch.rsp) }
    }

    pub fn switch_to(_old: &Task, _new: &Task) {
        todo!()
    }
}
