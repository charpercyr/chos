use chos_lib::arch::intr::wait_for_interrupt;
use chos_lib::arch::regs::Rsp;
use chos_lib::log::{debug, println};

use super::{current_task_arc, schedule, Task, TaskArc, TaskOps, CURRENT_TASK};
use crate::mm::virt::stack::alloc_kernel_stack;
use crate::mm::{per_cpu_lazy, PerCpu};

#[cfg(debug_assertions)]
const IDLE_STACK_ORDER: u8 = 2;
#[cfg(not(debug_assertions))]
const IDLE_STACK_ORDER: u8 = 0;

static IDLE_TASK_OPS: TaskOps = TaskOps {
    wake: |_| panic!("Should never be awakened"),
};

per_cpu_lazy! {
    static mut ref IDLE_TASK: TaskArc = {
        let stack = alloc_kernel_stack(IDLE_STACK_ORDER).expect("Stack alloc should not fail");
        debug!("Using {:#x}-{:#x} for idle stack", stack.range.start(), stack.range.end());
        Task::with_fn(
            stack,
            idle_loop,
            "[idle]",
            &IDLE_TASK_OPS,
            None
        ).expect("Idle Task::new() should not fail")
    };
}

fn idle_loop() -> ! {
    loop {
        wait_for_interrupt();
        schedule();
    }
}

pub(super) fn task() -> TaskArc {
    IDLE_TASK.clone()
}
