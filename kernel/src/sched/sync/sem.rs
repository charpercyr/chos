use chos_lib::log::todo_warn;
use chos_lib::sync::sem::TrySem;
use chos_lib::sync::{Sem, SpinSem};

use crate::sched::{current_task_arc, Task};

const TRIES: usize = 64;

pub struct SchedSem {
    inner: SpinSem,
}

impl SchedSem {
    fn block_task(&self, count: usize) {
        todo_warn!("Add task to waitlist with count {}", count);
        Task::mark_blocked_and_schedule(current_task_arc())
    }
}

impl Sem for SchedSem {
    fn with_count(count: usize) -> Self {
        Self {
            inner: SpinSem::with_count(count),
        }
    }

    fn wait_count(&self, count: usize) {
        if self.inner.try_wait_count_tries(count, TRIES) {
            return;
        }
        self.block_task(count);
    }

    fn signal_count(&self, count: usize) {
        self.inner.signal_count(count)
    }
}

impl TrySem for SchedSem {
    fn try_wait_count(&self, count: usize) -> bool {
        self.inner.try_wait_count(count)
    }
}
