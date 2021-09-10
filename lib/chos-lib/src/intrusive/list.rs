use core::cell::{Cell, UnsafeCell};
use core::mem::MaybeUninit;
use core::ptr::{replace, NonNull};

use super::{Adapter, LinkOps, Pointer};

pub struct Link<M> {
    next: Cell<Option<NonNull<Self>>>,
    prev: Cell<Option<NonNull<Self>>>,
    meta: UnsafeCell<MaybeUninit<M>>,
}

impl<M> Link<M> {
    pub const fn new() -> Self {
        Self {
            next: Cell::new(Self::PTR_UNLINKED),
            prev: Cell::new(Self::PTR_UNLINKED),
            meta: UnsafeCell::new(MaybeUninit::uninit()),
        }
    }
    pub const UNLINKED: Self = Self::new();
    const PTR_UNLINKED: Option<NonNull<Self>> = Some(NonNull::dangling());

    pub fn is_unlinked(&self) -> bool {
        self.prev.get() == Self::PTR_UNLINKED && self.next.get() == Self::PTR_UNLINKED
    }
}

pub trait ListLinkOps: LinkOps {
    fn get_prev(&self) -> Option<NonNull<Self>>;
    fn get_next(&self) -> Option<NonNull<Self>>;
    fn set_prev(&self, prev: Option<NonNull<Self>>);
    fn set_next(&self, next: Option<NonNull<Self>>);
}

impl<M> LinkOps for Link<M> {
    type Metadata = M;
    fn acquire(&self) -> bool {
        if self.is_unlinked() {
            self.next.set(None);
            self.prev.set(None);
            true
        } else {
            false
        }
    }
    fn release(&self) {
        self.next.set(Self::PTR_UNLINKED);
        self.prev.set(Self::PTR_UNLINKED);
    }

    unsafe fn set_meta(&self, meta: M) {
        *self.meta.get() = MaybeUninit::new(meta);
    }

    unsafe fn take_meta(&self) -> M {
        replace(self.meta.get(), MaybeUninit::uninit()).assume_init()
    }
}

impl<M> ListLinkOps for Link<M> {
    fn get_prev(&self) -> Option<NonNull<Self>> {
        self.prev.get()
    }
    fn get_next(&self) -> Option<NonNull<Self>> {
        self.next.get()
    }
    fn set_prev(&self, prev: Option<NonNull<Self>>) {
        self.prev.set(prev);
    }
    fn set_next(&self, next: Option<NonNull<Self>>) {
        self.next.set(next);
    }
}

pub struct HList<M, A: Adapter<Pointer: Pointer<Metadata = M>, Link: ListLinkOps<Metadata = M>>> {
    head: Option<NonNull<A::Link>>,
    adapter: A,
}

impl<M, A: Adapter<Pointer: Pointer<Metadata = M>, Link: ListLinkOps<Metadata = M>>> HList<M, A> {
    pub const fn new(adapter: A) -> Self {
        Self {
            head: None,
            adapter,
        }
    }

    fn update_head(&mut self, node: NonNull<A::Link>) {
        self.head = Some(node);
    }

    unsafe fn update_unlink(&mut self, node: NonNull<A::Link>) {
        let link = node.as_ref();
        if self.head == Some(node) {
            self.head = link.get_next();
        }
    }
}

pub struct List<M, A: Adapter<Pointer: Pointer<Metadata = M>, Link: ListLinkOps<Metadata = M>>> {
    head: Option<NonNull<A::Link>>,
    tail: Option<NonNull<A::Link>>,
    adapter: A,
}

impl<M, A: Adapter<Pointer: Pointer<Metadata = M>, Link: ListLinkOps<Metadata = M>>> List<M, A> {
    pub const fn new(adapter: A) -> Self {
        Self {
            head: None,
            tail: None,
            adapter,
        }
    }

    pub fn try_push_back(&mut self, ptr: A::Pointer) -> Result<(), A::Pointer> {
        unsafe {
            let (value, meta) = A::Pointer::into_raw(ptr);
            let link = self.adapter.get_link(value);
            let link = NonNull::new_unchecked(link as *mut A::Link);
            if let Some(tail) = &mut self.tail {
                if !insert_after(link, *tail) {
                    return Err(A::Pointer::from_raw(
                        self.adapter.get_value(link.as_ptr()),
                        meta,
                    ));
                }
            } else {
                if !link.as_ref().acquire() {
                    return Err(A::Pointer::from_raw(
                        self.adapter.get_value(link.as_ptr()),
                        meta,
                    ));
                }
            }
            link.as_ref().set_meta(meta);
            self.update_tail(link);
        }
        Ok(())
    }

    pub fn push_back(&mut self, ptr: A::Pointer) {
        if let Err(_) = self.try_push_back(ptr) {
            panic!("Could not insert: already linked");
        }
    }

    pub fn pop_back(&mut self) -> Option<A::Pointer> {
        self.back_mut().unlink()
    }

    pub fn back(&self) -> ListCursor<'_, M, A> {
        ListCursor {
            list: self,
            cur: self.tail,
        }
    }

    pub fn back_mut(&mut self) -> ListCursorMut<'_, M, A> {
        let cur = self.tail;
        ListCursorMut { list: self, cur }
    }

    fn update_head(&mut self, node: NonNull<A::Link>) {
        self.head = Some(node);
        if self.tail.is_none() {
            self.tail = Some(node);
        }
    }

    fn update_tail(&mut self, node: NonNull<A::Link>) {
        self.tail = Some(node);
        if self.head.is_none() {
            self.head = Some(node);
        }
    }

    unsafe fn update_unlink(&mut self, node: NonNull<A::Link>) {
        let link = node.as_ref();
        if self.head == Some(node) {
            self.head = link.get_next();
        }
        if self.tail == Some(node) {
            self.tail = link.get_prev();
        }
    }
}

unsafe fn insert_before<L: ListLinkOps>(node: NonNull<L>, before: NonNull<L>) -> bool {
    let link = node.as_ref();
    if !link.acquire() {
        return false;
    }
    link.set_next(Some(before));
    link.set_prev(before.as_ref().get_prev());
    before.as_ref().set_prev(Some(node));
    if let Some(prev) = link.get_prev() {
        prev.as_ref().set_next(Some(node));
    }
    true
}

unsafe fn insert_after<L: ListLinkOps>(node: NonNull<L>, after: NonNull<L>) -> bool {
    let link = node.as_ref();
    if !link.acquire() {
        return false;
    }
    link.set_prev(Some(after));
    link.set_next(after.as_ref().get_next());
    after.as_ref().set_next(Some(node));
    if let Some(next) = link.get_next() {
        next.as_ref().set_prev(Some(node));
    }
    true
}

unsafe fn unlink<L: ListLinkOps>(node: NonNull<L>) {
    let link = node.as_ref();
    if let Some(next) = link.get_next() {
        next.as_ref().set_prev(link.get_prev());
    }
    if let Some(prev) = link.get_prev() {
        prev.as_ref().set_next(link.get_next());
    }
    link.set_next(None);
    link.set_prev(None);
    link.release();
}

macro_rules! cursor_common_impl {
    ($list:ident, $cursor:ident) => {
        impl<
                'a,
                M,
                A: Adapter<Pointer: Pointer<Metadata = M>, Link: ListLinkOps<Metadata = M>>,
            > $cursor<'a, M, A>
        {
            pub fn get(&self) -> Option<&'a A::Value> {
                Some(unsafe { &*self.list.adapter.get_value(self.cur?.as_ptr()) })
            }

            pub fn move_next(&mut self) {
                if let Some(cur) = self.cur {
                    self.cur = unsafe { cur.as_ref().get_next() };
                }
            }

            pub fn move_prev(&mut self) {
                if let Some(cur) = self.cur {
                    self.cur = unsafe { cur.as_ref().get_prev() };
                }
            }

            pub fn is_valid(&self) -> bool {
                self.cur.is_some()
            }
        }
    };
}

macro_rules! list_common_impl {
    ($list:ident, $cursor:ident, $cursor_mut:ident, $iter:ident) => {
        impl<M, A: Adapter<Pointer: Pointer<Metadata = M>, Link: ListLinkOps<Metadata = M>>>
            $list<M, A>
        {
            pub fn try_push_front(&mut self, ptr: A::Pointer) -> Result<(), A::Pointer> {
                unsafe {
                    let (value, meta) = A::Pointer::into_raw(ptr);
                    let link = self.adapter.get_link(value);
                    let link = NonNull::new_unchecked(link as *mut A::Link);
                    if let Some(head) = &mut self.head {
                        if !insert_before(link, *head) {
                            return Err(A::Pointer::from_raw(
                                self.adapter.get_value(link.as_ptr()),
                                meta,
                            ));
                        }
                    } else {
                        if !link.as_ref().acquire() {
                            return Err(A::Pointer::from_raw(
                                self.adapter.get_value(link.as_ptr()),
                                meta,
                            ));
                        }
                    }
                    link.as_ref().set_meta(meta);
                    self.update_head(link);
                }
                Ok(())
            }

            pub fn push_front(&mut self, ptr: A::Pointer) {
                if let Err(_) = self.try_push_front(ptr) {
                    panic!("Could not insert: already linked");
                }
            }

            pub fn pop_front(&mut self) -> Option<A::Pointer> {
                self.front_mut().unlink()
            }

            pub fn front(&self) -> $cursor<'_, M, A> {
                $cursor {
                    list: self,
                    cur: self.head,
                }
            }

            pub fn front_mut(&mut self) -> $cursor_mut<'_, M, A> {
                let head = self.head;
                $cursor_mut {
                    list: self,
                    cur: head,
                }
            }

            pub fn iter(&self) -> $iter<'_, M, A> {
                $iter {
                    cursor: self.front(),
                }
            }

            pub unsafe fn cursor_from_pointer(&self, ptr: *const A::Value) -> $cursor<'_, M, A> {
                let cur = self.adapter.get_link(ptr);
                $cursor {
                    list: self,
                    cur: Some(NonNull::new_unchecked(cur as *mut A::Link)),
                }
            }

            pub unsafe fn cursor_mut_from_pointer(
                &mut self,
                ptr: *const A::Value,
            ) -> $cursor<'_, M, A> {
                let cur = self.adapter.get_link(ptr);
                $cursor {
                    list: self,
                    cur: Some(NonNull::new_unchecked(cur as *mut A::Link)),
                }
            }
        }

        impl<M, A: Adapter<Pointer: Pointer<Metadata = M>, Link: ListLinkOps<Metadata = M>>> Drop
            for $list<M, A>
        {
            fn drop(&mut self) {
                while let Some(_) = self.pop_front() {
                    // Drop all pointers still in the list
                }
            }
        }

        pub struct $cursor<
            'a,
            M,
            A: Adapter<Pointer: Pointer<Metadata = M>, Link: ListLinkOps<Metadata = M>>,
        > {
            list: &'a $list<M, A>,
            cur: Option<NonNull<A::Link>>,
        }

        pub struct $cursor_mut<
            'a,
            M,
            A: Adapter<Pointer: Pointer<Metadata = M>, Link: ListLinkOps<Metadata = M>>,
        > {
            list: &'a mut $list<M, A>,
            cur: Option<NonNull<A::Link>>,
        }

        impl<
                'a,
                M,
                A: Adapter<Pointer: Pointer<Metadata = M>, Link: ListLinkOps<Metadata = M>>,
            > $cursor_mut<'a, M, A>
        {
            pub fn unlink(self) -> Option<A::Pointer> {
                self.cur.map(|cur| unsafe {
                    self.list.update_unlink(cur);
                    unlink(cur);
                    let meta = cur.as_ref().take_meta();
                    A::Pointer::from_raw(self.list.adapter.get_value(cur.as_ptr()), meta)
                })
            }
        }

        cursor_common_impl!($list, $cursor);
        cursor_common_impl!($list, $cursor_mut);

        pub struct $iter<
            'a,
            M,
            A: Adapter<Pointer: Pointer<Metadata = M>, Link: ListLinkOps<Metadata = M>>,
        > {
            cursor: $cursor<'a, M, A>,
        }

        impl<
                'a,
                M,
                A: Adapter<Pointer: Pointer<Metadata = M>, Link: ListLinkOps<Metadata = M>>,
            > Iterator for $iter<'a, M, A>
        {
            type Item = &'a A::Value;
            fn next(&mut self) -> Option<Self::Item> {
                let res = self.cursor.get();
                self.cursor.move_next();
                res
            }
        }

        impl<
                'a,
                M,
                A: Adapter<Pointer: Pointer<Metadata = M>, Link: ListLinkOps<Metadata = M>>,
            > IntoIterator for &'a $list<M, A>
        {
            type Item = &'a A::Value;
            type IntoIter = $iter<'a, M, A>;
            fn into_iter(self) -> Self::IntoIter {
                self.iter()
            }
        }
    };
}

list_common_impl!(HList, HListCursor, HListCursorMut, HListIter);
list_common_impl!(List, ListCursor, ListCursorMut, ListIter);

#[cfg(test)]
mod tests {
    use std::prelude::v1::*;

    use super::*;

    struct A {
        link: Link<()>,
        value: u32,
    }
    impl A {
        fn new(value: u32) -> Self {
            Self {
                link: Link::UNLINKED,
                value,
            }
        }
    }

    crate::intrusive_adapter!(struct AAdapter<'a> = &'a A: A { link: Link<()> });

    fn with_list(f: impl FnOnce(&mut List<(), AAdapter>)) {
        let a0 = A::new(0);
        let a1 = A::new(1);
        let a2 = A::new(2);
        let mut list = List::new(AAdapter::new());
        list.push_back(&a0);
        list.push_back(&a1);
        list.push_back(&a2);
        f(&mut list);
    }

    fn with_hlist(f: impl FnOnce(&mut HList<(), AAdapter>)) {
        let a0 = A::new(0);
        let a1 = A::new(1);
        let a2 = A::new(2);
        let mut list = HList::new(AAdapter::new());
        list.push_front(&a2);
        list.push_front(&a1);
        list.push_front(&a0);
        f(&mut list);
    }

    #[test]
    fn list_push_front() {
        let a0 = A::new(0);
        let a1 = A::new(1);
        let a2 = A::new(2);
        let mut list = List::new(AAdapter::new());
        list.push_front(&a0);
        list.push_front(&a1);
        list.push_front(&a2);
        let res: Vec<_> = list.iter().collect();
        assert_eq!(res[0].value, 2);
        assert_eq!(res[1].value, 1);
        assert_eq!(res[2].value, 0);
    }

    #[test]
    fn list_push_back() {
        let a0 = A::new(0);
        let a1 = A::new(1);
        let a2 = A::new(2);
        let mut list = List::new(AAdapter::new());
        list.push_back(&a0);
        list.push_back(&a1);
        list.push_back(&a2);
        let res: alloc::vec::Vec<_> = list.iter().collect();
        assert_eq!(res[0].value, 0);
        assert_eq!(res[1].value, 1);
        assert_eq!(res[2].value, 2);
    }

    #[test]
    fn list_pop_front() {
        with_list(|list| {
            assert_eq!(list.pop_front().map(|a| a.value), Some(0));
            assert_eq!(list.pop_front().map(|a| a.value), Some(1));
            assert_eq!(list.pop_front().map(|a| a.value), Some(2));
            assert_eq!(list.pop_front().map(|a| a.value), None);
        });
    }

    #[test]
    fn list_pop_back() {
        with_list(|list| {
            assert_eq!(list.pop_back().map(|a| a.value), Some(2));
            assert_eq!(list.pop_back().map(|a| a.value), Some(1));
            assert_eq!(list.pop_back().map(|a| a.value), Some(0));
            assert_eq!(list.pop_back().map(|a| a.value), None);
        });
    }

    #[test]
    fn list_cursor_unlink() {
        with_list(|list| {
            let mut c = list.front_mut();
            c.move_next();
            c.unlink();
            let res: Vec<_> = list.iter().collect();
            assert_eq!(res[0].value, 0);
            assert_eq!(res[1].value, 2);
        })
    }

    #[test]
    fn list_try_push_front_dup() {
        let a = A::new(0);
        let mut list = List::new(AAdapter::new());
        assert!(list.try_push_front(&a).is_ok());
        assert!(list.try_push_front(&a).is_err());
    }

    #[test]
    fn list_try_push_back_dup() {
        let a = A::new(0);
        let mut list = List::new(AAdapter::new());
        assert!(list.try_push_back(&a).is_ok());
        assert!(list.try_push_back(&a).is_err());
    }

    #[test]
    fn hlist_push_front() {
        let a0 = A::new(0);
        let a1 = A::new(1);
        let a2 = A::new(2);
        let mut list = HList::new(AAdapter::new());
        list.push_front(&a0);
        list.push_front(&a1);
        list.push_front(&a2);
        let res: Vec<_> = list.iter().collect();
        assert_eq!(res[0].value, 2);
        assert_eq!(res[1].value, 1);
        assert_eq!(res[2].value, 0);
    }

    #[test]
    fn hlist_pop_front() {
        with_hlist(|list| {
            assert_eq!(list.pop_front().map(|a| a.value), Some(0));
            assert_eq!(list.pop_front().map(|a| a.value), Some(1));
            assert_eq!(list.pop_front().map(|a| a.value), Some(2));
            assert_eq!(list.pop_front().map(|a| a.value), None);
        });
    }

    #[test]
    fn hlist_cursor_unlink() {
        with_hlist(|list| {
            let mut c = list.front_mut();
            c.move_next();
            c.unlink();
            let res: Vec<_> = list.iter().collect();
            assert_eq!(res[0].value, 0);
            assert_eq!(res[1].value, 2);
        })
    }

    #[test]
    fn hlist_try_push_front_dup() {
        let a = A::new(0);
        let mut list = HList::new(AAdapter::new());
        assert!(list.try_push_front(&a).is_ok());
        assert!(list.try_push_front(&a).is_err());
    }
}