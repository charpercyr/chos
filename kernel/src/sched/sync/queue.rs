use chos_lib::sync::{Spinlock, Sem};
use intrusive_collections::{linked_list, Adapter, PointerOps};

use super::SchedSem;

pub struct SchedQueue<A: Adapter<LinkOps: linked_list::LinkedListOps>> {
    list: Spinlock<linked_list::LinkedList<A>>,
    sem: SchedSem,
}

impl<A: Adapter<LinkOps: linked_list::LinkedListOps>> SchedQueue<A> {
    pub const fn new(adapter: A) -> Self {
        Self {
            list: Spinlock::new(linked_list::LinkedList::new(adapter)),
            sem: SchedSem::zero(),
        }
    }

    pub fn push(&self, ptr: <A::PointerOps as PointerOps>::Pointer) {
        {
            let mut list = self.list.lock();
            list.push_back(ptr);
        }
        self.sem.signal();
    }

    pub fn pop_wait(&self) -> <A::PointerOps as PointerOps>::Pointer {
        self.sem.wait();
        let mut list = self.list.lock();
        list.pop_front().unwrap()
    }

    pub fn find_pop_wait(&self, mut filter: impl FnMut(&<A::PointerOps as PointerOps>::Value) -> bool) -> Option<<A::PointerOps as PointerOps>::Pointer> {
        self.sem.wait();
        {
            let mut list = self.list.lock();
            let mut cursor = list.front_mut();
            while let Some(value) = cursor.get() {
                if filter(value) {
                    return Some(cursor.remove().unwrap());
                }
                cursor.move_next();
            }
        }
        self.sem.signal(); // We ended not taking an element from the list
        None
    }
}
