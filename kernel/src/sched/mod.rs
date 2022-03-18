mod idle;
mod ktask;

use alloc::borrow::Cow;
use core::hint::black_box;
use core::intrinsics::likely;
use core::ptr::NonNull;
use core::sync::atomic::AtomicBool;

use chos_lib::arch::mm::VAddr;
use chos_lib::init::ConstInit;
use chos_lib::log::debug;
use chos_lib::pool::{IArc, IArcAdapter, IArcCount};
use intrusive_collections::LinkedListAtomicLink;

use crate::arch::sched::ArchTask;
use crate::mm::slab::DefaultPoolObjectAllocator;
use crate::mm::{per_cpu, PerCpu};

pub struct Task {
    link: LinkedListAtomicLink,
    count: IArcCount,
    kernel_stack: VAddr,
    arch: ArchTask,
    data: Option<NonNull<()>>,
    debug_name: Cow<'static, str>,
}
unsafe impl Send for Task {}
unsafe impl Sync for Task {}

impl Task {
    fn new(
        kernel_stack: VAddr,
        data: Option<NonNull<()>>,
        debug_name: impl Into<Cow<'static, str>>,
    ) -> Option<TaskArc> {
        Some(TaskArc::new(Task {
            link: LinkedListAtomicLink::new(),
            count: IArcCount::INIT,
            kernel_stack,
            arch: ArchTask::new(),
            data,
            debug_name: debug_name.into(),
        }))
    }

    pub fn debug_name(&self) -> Option<&str> {
        self.debug_name.as_ref().into()
    }
}

static TASK_POOL: DefaultPoolObjectAllocator<Task, 0> = ConstInit::INIT;
chos_lib::pool!(pub struct TaskPool: Task => &TASK_POOL);

impl IArcAdapter for Task {
    fn count(&self) -> &IArcCount {
        &self.count
    }
}
pub type TaskArc = IArc<Task, TaskPool>;

chos_lib::intrusive_adapter!(TaskAdapter = TaskArc: Task { link: LinkedListAtomicLink });

pub fn schedule() {
    const SCHEDULERS: [fn() -> Option<TaskArc>; 2] = [ktask::find_next_task, idle::find_next_task];
    let mut next_task = None;
    for sched in &SCHEDULERS {
        if let Some(task) = sched() {
            next_task = Some(task);
            break;
        }
    }
    let next_task = next_task.expect("Should always have a task to schedule");
    debug!(
        "Scheduling task name '{}'",
        next_task.debug_name().unwrap_or("<unknown>")
    );
    drop(black_box(next_task))
}

pub fn enter_schedule() -> ! {
    IN_SCHED.store(true, core::sync::atomic::Ordering::Relaxed);
    debug!("enter_schedule()");
    schedule();
    loop {}
    // unreachable!();
}

pub fn schedule_tick() {
    if SCHED_DISABLE.copy() == 0 {
        // TODO
    }
}

static IN_SCHED: AtomicBool = AtomicBool::new(false);
per_cpu! {
    static mut ref SCHED_DISABLE: u64 = 0;
}
#[inline(always)]
fn in_sched() -> bool {
    IN_SCHED.load(core::sync::atomic::Ordering::Relaxed)
}

#[no_mangle]
#[inline]
fn __lock_disable_sched_save() -> u64 {
    if likely(in_sched()) {
        SCHED_DISABLE.with(|v| *v += 1);
    }
    0
}

#[no_mangle]
#[inline]
fn __lock_restore_sched(_: u64) {
    if likely(in_sched()) {
        SCHED_DISABLE.with(|v| *v -= 1);
    }
}
