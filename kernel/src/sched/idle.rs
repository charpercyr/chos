use chos_lib::arch::intr::wait_for_interrupt;
use chos_lib::log::debug;
use chos_lib::mm::VAddr;

use super::{Task, TaskArc};
use crate::mm::virt::stack::alloc_kernel_stack;
use crate::mm::{per_cpu_lazy, PerCpu};

per_cpu_lazy! {
    static mut ref IDLE_STACK: VAddr = {
        let stack = alloc_kernel_stack(0).expect("Stack alloc should not fail");
        let stack = stack.range.end().addr();
        debug!("Using {:#x} for idle stack", stack);
        stack
    };
    static mut ref IDLE_TASK: TaskArc = Task::new(
        IDLE_STACK.copy(),
        None,
        "[idle]",
    ).expect("Idle Task::new() should not fail");
}

fn idle() -> ! {
    loop {
        wait_for_interrupt();
    }
}

pub fn find_next_task() -> Option<TaskArc> {
    Some(IDLE_TASK.clone())
}
