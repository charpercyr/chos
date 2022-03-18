use chos_lib::arch::intr::wait_for_interrupt;
use chos_lib::arch::mm::VAddr;
use chos_lib::log::debug;
use chos_lib::sync::SpinLazy;

use super::{Task, TaskArc};
use crate::cpumask::cpu_count;
use crate::mm::stack::{allocate_kernel_stacks_order, Stacks};
use crate::mm::{per_cpu_lazy, PerCpu};

static IDLE_STACKS_STRUCT: SpinLazy<Stacks> =
    SpinLazy::new(|| unsafe { allocate_kernel_stacks_order(cpu_count(), 0) });

per_cpu_lazy! {
    static mut ref IDLE_STACK: VAddr = {
        let (base, size) = IDLE_STACKS_STRUCT.get().get_for_this_cpu();
        let stack = base + size;
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
