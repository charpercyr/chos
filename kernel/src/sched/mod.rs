use chos_lib::{init::ConstInit, pool::PoolBox};
use intrusive_collections::{LinkedList, LinkedListAtomicLink};

use crate::{mm::slab::{DefaultPoolObjectAllocator}, per_cpu};

const N_PRIO: usize = 16;

pub struct Task {
    link: LinkedListAtomicLink,
}
static TASK_POOL: DefaultPoolObjectAllocator<Task, 0> = ConstInit::INIT;
chos_lib::pool!(struct TaskPool: Task => &TASK_POOL);
type TaskBox = PoolBox<Task, TaskPool>;
chos_lib::intrusive_adapter!(TaskAdapter = TaskBox: Task { link: LinkedListAtomicLink });

struct SchedQueue {
    prios: [LinkedList<TaskAdapter>; N_PRIO],
}
impl ConstInit for SchedQueue {
    const INIT: Self = Self {
        prios: ConstInit::INIT,
    };
}

static GLOBAL_SCHED_QUEUE: SchedQueue = ConstInit::INIT;
per_cpu! {
    static mut ref SCHED_QUEUE: SchedQueue = ConstInit::INIT;
}

pub fn enter_schedule() -> ! {
    loop {
        todo!();
    }
}