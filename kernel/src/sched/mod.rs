mod idle;
pub mod ktask;
pub mod sync;

use alloc::borrow::Cow;
use core::intrinsics::likely;
use core::ptr::NonNull;
use core::sync::atomic::AtomicBool;

use chos_lib::init::ConstInit;
use chos_lib::log::debug;
use chos_lib::pool::{IArc, IArcAdapter, IArcCount};
use intrusive_collections::LinkedListAtomicLink;

use crate::arch::sched::ArchTask;
use crate::mm::slab::DefaultPoolObjectAllocator;
use crate::mm::virt::stack::Stack;
use crate::mm::{per_cpu, PerCpu};

pub struct TaskOps {
    pub wake: fn(&Task),
}

pub struct TaskNode {
    link: LinkedListAtomicLink,
}

pub struct Task {
    link: LinkedListAtomicLink,
    count: IArcCount,
    debug_name: Cow<'static, str>,
    pub arch: ArchTask,
    data: Option<NonNull<()>>,
    ops: &'static TaskOps,
}
unsafe impl Send for Task {}
unsafe impl Sync for Task {}

per_cpu! {
    static mut ref CURRENT_TASK: Option<TaskArc> = None;
}
fn current_task() -> TaskArc {
    #[cfg(debug_assertions)]
    return CURRENT_TASK.clone().expect("Not in schedule");
    #[cfg(not(debug_assertions))]
    return unsafe { CURRENT_TASK.unwrap_unchecked() };
}

impl Task {
    fn with_fn(
        kernel_stack: Stack,
        fun: fn() -> !,
        debug_name: impl Into<Cow<'static, str>>,
        ops: &'static TaskOps,
        data: Option<NonNull<()>>,
    ) -> Option<TaskArc> {
        Some(
            TaskArc::try_new(Task {
                link: LinkedListAtomicLink::new(),
                count: IArcCount::INIT,
                debug_name: debug_name.into(),
                arch: ArchTask::with_fn(kernel_stack, fun),
                data,
                ops,
            })
            .ok()?,
        )
    }

    pub fn debug_name(&self) -> Option<&str> {
        self.debug_name.as_ref().into()
    }

    pub fn enter_first_task(&self) -> ! {
        ArchTask::enter_first_task(self)
    }

    pub fn wake(&self) {
        (self.ops.wake)(self)
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

fn find_next_task() -> TaskArc {
    const SCHEDULERS: [fn() -> Option<TaskArc>; 2] = [ktask::find_next_task, idle::find_next_task];
    SCHEDULERS
        .iter()
        .find_map(|scheduler| scheduler())
        .expect("Should always have a task to schedule")
}

pub fn schedule() {
    todo!()
}

pub fn enter_schedule() -> ! {
    IN_SCHED.store(true, core::sync::atomic::Ordering::Relaxed);
    debug!("enter_schedule()");
    let task = find_next_task();
    CURRENT_TASK.with(|cur| {
        debug_assert!(cur.is_none());
        *cur = Some(task.clone());
    });
    task.enter_first_task();
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
        true
    } else {
        false
    }
    .into()
}

#[no_mangle]
#[inline]
fn __lock_restore_sched(in_sched: u64) {
    let in_sched = match in_sched {
        0 => false,
        1 => true,
        _ => panic!("Invalid value for in_sched"),
    };
    if likely(in_sched) {
        SCHED_DISABLE.with(|v| *v -= 1);
    }
}
