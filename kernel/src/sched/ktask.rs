use alloc::borrow::Cow;
use core::future::Future;

use chos_lib::arch::mm::VAddr;
use chos_lib::log::debug;
use chos_lib::sync::SpinLazy;

use super::{Task, TaskArc};
use crate::cpumask::cpu_count;
use crate::mm::per_cpu_lazy;
use crate::mm::stack::{allocate_kernel_stacks, Stacks};

static KTASK_STACKS_STRUCT: SpinLazy<Stacks> =
    SpinLazy::new(|| unsafe { allocate_kernel_stacks(cpu_count()) });

per_cpu_lazy! {
    static mut ref KTASK_STACK: VAddr = {
        let (base, size) = KTASK_STACKS_STRUCT.get().get_for_this_cpu();
        let stack = base + size;
        debug!("Using {:#x} for ktask stack", stack);
        stack
    };
    static mut ref KTASK_TASK: TaskArc = Task::new(
        KTASK_STACK.copy(),
        None,
        "[ktask]",
    ).expect("Ktask Task::new() should not fail");
}

pub fn find_next_task() -> Option<TaskArc> {
    None
}

pub fn spawn(_f: impl FnOnce() + Send + 'static, name: Cow<'static, str>) {
    todo!("Spawn function {}", name)
}

pub fn spawn_future(_f: impl Future<Output = ()> + Send + 'static, name: Cow<'static, str>) {
    todo!("Spawn future {}", name)
}
