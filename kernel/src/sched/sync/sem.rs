use chos_lib::sync::sem::TrySem;
use chos_lib::sync::{Sem, SpinSem, Spinlock};
use intrusive_collections::{intrusive_adapter, linked_list, UnsafeMut};

use crate::sched::{current_task_arc, Task, TaskArc};

const TRIES: usize = 64;

struct WaitNode {
    link: linked_list::AtomicLink,
    count: usize,
    task: Option<TaskArc>,
}
intrusive_adapter!(WaitNodeAdapter = UnsafeMut<WaitNode>: WaitNode { link: linked_list::AtomicLink });

pub struct SchedSem {
    inner: SpinSem,
    waitlist: Spinlock<linked_list::LinkedList<WaitNodeAdapter>>,
}

impl SchedSem {
    pub const fn zero() -> Self {
        Self::with_count(0)
    }

    pub const fn with_count(count: usize) -> Self {
        Self {
            inner: SpinSem::with_count(count),
            waitlist: Spinlock::new(linked_list::LinkedList::new(WaitNodeAdapter::NEW)),
        }
    }

    fn block_task(&self, count: usize) {
        let task = current_task_arc();
        let mut node = WaitNode {
            link: linked_list::AtomicLink::new(),
            count,
            task: Some(task.clone()),
        };
        {
            let mut waitlist = self.waitlist.lock();
            waitlist.push_back(unsafe { UnsafeMut::from_raw(&mut node) });
        }
        Task::mark_blocked_and_schedule(task);
        assert!(!node.link.is_linked());
        drop(node);
    }
}

impl Sem for SchedSem {
    fn wait_count(&self, count: usize) {
        loop {
            if self.inner.try_wait_count_tries(count, TRIES) {
                return;
            }
            self.block_task(count);
        }
    }

    fn signal_count(&self, mut count: usize) {
        self.inner.signal_count(count);
        let mut waitlist = self.waitlist.lock();
        let mut cursor = waitlist.front_mut();
        while count > 0 {
            if let Some(cur) = cursor.get() {
                if cur.count <= count {
                    let mut node = cursor.remove().unwrap();
                    count -= node.count;
                    let task = node.task.take().unwrap();
                    drop(node);
                    Task::wake(task);
                } else {
                    cursor.move_next();
                }
            } else {
                return;
            }
        }
    }
}

impl TrySem for SchedSem {
    fn try_wait_count(&self, count: usize) -> bool {
        self.inner.try_wait_count(count)
    }
}
