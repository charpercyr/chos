use chos_lib::init::ConstInit;
use chos_lib::pool::PoolBox;
use intrusive_collections::{LinkedList, LinkedListAtomicLink};

use crate::mm::per_cpu;
use crate::mm::slab::DefaultPoolObjectAllocator;

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
    todo!("enter_schedule()")
}
