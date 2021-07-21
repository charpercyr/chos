
use super::{Adapter, Pointer};

use core::cell::Cell;
use core::fmt;
use core::ptr::null;

pub struct Link {
    next: Cell<*const Self>,
}
impl Link {
    pub const UNLINKED: Self = Self {
        next: Cell::new(null()),
    };

    pub fn is_unlinked(&self) -> bool {
        self.next.get() == null()
    }
}
unsafe impl Send for Link {}

impl fmt::Debug for Link {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Link").finish()
    }
}

pub struct SList<A: Adapter<Link = Link>> {
    head: *const Link,
    adapter: A,
}
impl<A: Adapter<Link = Link>> SList<A> {
    pub const fn new(adapter: A) -> Self {
        Self {
            head: null(),
            adapter,
        }
    }

    pub fn push_front(&mut self, ptr: A::Pointer) {
        unsafe {
            let link = &*self.adapter.get_link(A::Pointer::into_raw(ptr));
            assert!(link.is_unlinked());
            link.next.set(self.head);
            self.head = link;
        }
    }

    pub fn pop_front(&mut self) -> Option<A::Pointer> {
        unsafe {
            if self.head == null() {
                None
            } else {
                let link = &*self.head;
                self.head = link.next.get();
                link.next.set(null());
                Some(A::Pointer::from_raw(self.adapter.get_value(link)))
            }
        }
    }

    pub fn front(&self) -> Cursor<A> {
        Cursor {
            list: self,
            cur: self.head,
        }
    }

    pub fn iter(&self) -> Iter<A> {
        Iter {
            cursor: self.front(),
        }
    }
}

unsafe impl<A: Adapter<Link = Link> + Send> Send for SList<A> {}
unsafe impl<A: Adapter<Link = Link> + Sync> Sync for SList<A> {}

pub struct Cursor<'a, A: Adapter<Link = Link>> {
    list: &'a SList<A>,
    cur: *const Link,
}
impl<'a, A: Adapter<Link = Link>> Cursor<'a, A> {
    pub fn move_next(&mut self) {
        unsafe {
            if self.is_valid() {
                if self.is_tail() {
                    self.cur = (*self.cur).next.get();
                } else {
                    self.cur = null();
                }
            } 
        }
    }

    pub fn try_get_ref(&self) -> Option<&'a A::Value> {
        (self.cur != null()).then(|| unsafe { &*self.list.adapter.get_value(self.cur) })
    }

    pub fn is_tail(&self) -> bool {
        unsafe { self.is_valid() && (*self.cur).next.get() != null() }
    }

    pub fn is_valid(&self) -> bool {
        self.cur != null()
    }
}

impl<'a, A: Adapter<Link = Link>> Clone for Cursor<'a, A> {
    fn clone(&self) -> Self {
        Self {
            list: self.list,
            cur: self.cur,
        }
    }
}
impl<'a, A: Adapter<Link = Link>> Copy for Cursor<'a, A> {}

pub struct Iter<'a, A: Adapter<Link = Link>> {
    cursor: Cursor<'a, A>,
}
impl<'a, A: Adapter<Link = Link>> Iterator for Iter<'a, A> {
    type Item = &'a A::Value;
    fn next(&mut self) -> Option<Self::Item> {
        let r = self.cursor.try_get_ref();
        self.cursor.move_next();
        r
    }
}
impl<'a, A: Adapter<Link = Link>> IntoIterator for &'a SList<A> {
    type Item = &'a A::Value;
    type IntoIter = Iter<'a, A>;
    fn into_iter(self) -> Self::IntoIter {
        self.iter()
    }
}
