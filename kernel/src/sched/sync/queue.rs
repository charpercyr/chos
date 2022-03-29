use alloc::collections::VecDeque;

use chos_lib::sync::sem::TrySem;
use chos_lib::sync::{Sem, Spinlock};

use super::SchedSem;

pub struct SchedQueue<T> {
    sem: SchedSem,
    data: Spinlock<VecDeque<T>>,
}
unsafe impl<T: Send> Send for SchedQueue<T> {}
unsafe impl<T: Sync> Sync for SchedQueue<T> {}

impl<T> SchedQueue<T> {
    pub fn push(&self, value: T) {
        {
            let mut data = self.data.lock();
            data.push_back(value);
        }
        self.sem.signal();
    }

    pub fn pop(&self) -> T {
        loop {
            self.sem.wait();
            let mut data = self.data.lock();
            break data
                .pop_front()
                .expect("Should have an element in the queue");
        }
    }

    pub fn try_pop(&self) -> Option<T> {
        self.sem.try_wait().then(|| self.do_pop())
    }

    pub fn try_pop_tries(&self, tries: usize) -> Option<T> {
        self.sem.try_wait_tries(tries).then(|| self.do_pop())
    }

    fn do_pop(&self) -> T {
        let mut data = self.data.lock();
        data.pop_front()
            .expect("Should have an element in the queue")
    }
}
