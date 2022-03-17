pub mod early;

use core::intrinsics::likely;
use core::sync::atomic::AtomicBool;

use chos_lib::init::ConstInit;
use chos_lib::log::println;
use chos_lib::pool::PoolBox;
use chos_lib::sync::Spinlock;
use intrusive_collections::{LinkedList, LinkedListAtomicLink};

use crate::mm::slab::DefaultPoolObjectAllocator;
use crate::mm::{per_cpu, PerCpu};

const N_PRIO: usize = 16;

pub struct Task {
    link: LinkedListAtomicLink,
}
static TASK_POOL: DefaultPoolObjectAllocator<Task, 0> = ConstInit::INIT;
chos_lib::pool!(pub struct TaskPool: Task => &TASK_POOL);
pub type TaskBox = PoolBox<Task, TaskPool>;
chos_lib::intrusive_adapter!(TaskAdapter = TaskBox: Task { link: LinkedListAtomicLink });

struct SchedQueue {
    prios: [LinkedList<TaskAdapter>; N_PRIO],
}
impl ConstInit for SchedQueue {
    const INIT: Self = Self {
        prios: ConstInit::INIT,
    };
}

static GLOBAL_SCHED_QUEUE: Spinlock<SchedQueue> = ConstInit::INIT;
per_cpu! {
    static mut ref SCHED_QUEUE: SchedQueue = ConstInit::INIT;
}

fn schedule() {}

pub fn enter_schedule() -> ! {
    IN_SCHED.store(true, core::sync::atomic::Ordering::Relaxed);
    println!("enter_schedule()");
    loop {
        schedule();
        unsafe { core::arch::asm!("hlt") };
    }
}

pub fn schedule_tick() {
    if SCHED_DISABLE.read() == 0 {}
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
