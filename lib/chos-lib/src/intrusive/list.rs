use core::cell::{Cell, UnsafeCell};
use core::fmt;
use core::marker::PhantomData;
use core::mem::{transmute, MaybeUninit};
use core::ptr::{null_mut, replace, NonNull};
use core::sync::atomic::{AtomicPtr, Ordering};

use super::{Adapter, ExclusivePointerOps, LinkOps, PointerOps};
use crate::init::ConstInit;
use crate::ptr::dangling_mut;

pub trait ListLinkOps: LinkOps {
    unsafe fn get_prev(&self) -> Option<NonNull<Self>>;
    unsafe fn get_next(&self) -> Option<NonNull<Self>>;
    unsafe fn set_prev(&self, prev: Option<NonNull<Self>>);
    unsafe fn set_next(&self, next: Option<NonNull<Self>>);
}

pub struct AtomicLink<M> {
    next: Cell<Option<NonNull<Self>>>,
    prev: Cell<Option<NonNull<Self>>>,
    meta: UnsafeCell<MaybeUninit<M>>,
}

impl<M> AtomicLink<M> {
    const PTR_UNLINKED: Option<NonNull<Self>> = Some(NonNull::dangling());
    const APTR_UNLINKED: *mut Self = dangling_mut();

    pub const fn new() -> Self {
        Self {
            next: Cell::new(Self::PTR_UNLINKED),
            prev: Cell::new(None),
            meta: UnsafeCell::new(MaybeUninit::uninit()),
        }
    }

    pub fn is_linked(&self) -> bool {
        self.atomic_next().load(Ordering::Relaxed) != Self::APTR_UNLINKED
    }

    pub unsafe fn force_unlink(&self) {
        self.release()
    }

    fn atomic_next(&self) -> &AtomicPtr<Self> {
        unsafe { transmute(&self.next) }
    }
}
impl<M> ConstInit for AtomicLink<M> {
    const INIT: Self = Self::new();
}
unsafe impl<M: Send> Send for AtomicLink<M> {}
unsafe impl<M: Sync> Sync for AtomicLink<M> {}

impl<M> fmt::Debug for AtomicLink<M> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("AtomicLink").finish()
    }
}
impl<M> Clone for AtomicLink<M> {
    fn clone(&self) -> Self {
        Self::new()
    }
}

impl<M> LinkOps for AtomicLink<M> {
    type Metadata = M;
    fn acquire(&self) -> bool {
        let next = self.atomic_next();
        loop {
            let value = next.load(Ordering::Relaxed);
            if value != Self::APTR_UNLINKED {
                return false;
            }
            if let Ok(_) = next.compare_exchange_weak(
                Self::APTR_UNLINKED,
                null_mut(),
                Ordering::Acquire,
                Ordering::Relaxed,
            ) {
                return true;
            }
        }
    }
    fn release(&self) {
        self.atomic_next()
            .store(Self::APTR_UNLINKED, Ordering::Release);
    }

    unsafe fn set_meta(&self, meta: Self::Metadata) {
        *self.meta.get() = MaybeUninit::new(meta)
    }

    unsafe fn take_meta(&self) -> Self::Metadata {
        replace(self.meta.get(), MaybeUninit::uninit()).assume_init()
    }
}

impl<M> ListLinkOps for AtomicLink<M> {
    unsafe fn get_prev(&self) -> Option<NonNull<Self>> {
        self.prev.get()
    }
    unsafe fn get_next(&self) -> Option<NonNull<Self>> {
        self.next.get()
    }
    unsafe fn set_prev(&self, prev: Option<NonNull<Self>>) {
        self.prev.set(prev);
    }
    unsafe fn set_next(&self, next: Option<NonNull<Self>>) {
        self.next.set(next);
    }
}

pub struct HList<A: Adapter<Link: ListLinkOps>> {
    head: Option<NonNull<A::Link>>,
    adapter: A,
    data: PhantomData<A::Pointer>,
}
unsafe impl<A: Adapter<Link: ListLinkOps> + Send> Send for HList<A> where A::Pointer: Send {}
unsafe impl<A: Adapter<Link: ListLinkOps> + Sync> Sync for HList<A> where A::Pointer: Sync {}

impl<A: Adapter<Link: ListLinkOps>> HList<A> {
    pub const fn new(adapter: A) -> Self {
        Self {
            head: None,
            adapter,
            data: PhantomData,
        }
    }

    pub fn try_push_front(&mut self, ptr: A::Pointer) -> Result<(), A::Pointer> {
        let (value, meta) = <A::Pointer as PointerOps>::into_raw(ptr);
        unsafe {
            let link = self.adapter.get_link(value);
            let link = NonNull::new_unchecked(link as *mut A::Link);
            if !link.as_ref().acquire() {
                return Err(<A::Pointer as PointerOps>::from_raw(value, meta));
            }
            link.as_ref().set_meta(meta);
            if let Some(head) = self.head {
                insert_before(head, link);
            }
            self.head = Some(link)
        }
        Ok(())
    }

    pub fn push_front(&mut self, ptr: A::Pointer) {
        if self.try_push_front(ptr).is_err() {
            panic!("Already linked");
        }
    }

    pub fn pop_front(&mut self) -> Option<A::Pointer> {
        self.front_mut().map(HListCursorMut::unlink)
    }

    pub fn front(&self) -> Option<HListCursor<'_, A>> {
        self.front_raw().map(|c| HListCursor {
            inner: c,
            list: self,
        })
    }

    pub fn front_mut(&mut self) -> Option<HListCursorMut<'_, A>> {
        self.front_raw().map(move |c| HListCursorMut {
            inner: c,
            list: self,
        })
    }

    pub fn iter(&self) -> HListIter<'_, A> {
        HListIter {
            cur: self.front_raw(),
            list: self,
        }
    }

    pub fn iter_mut(&mut self) -> HListIterMut<'_, A>
    where
        A::Pointer: ExclusivePointerOps,
    {
        HListIterMut {
            cur: self.front_raw(),
            list: self,
        }
    }

    pub fn into_iter(self) -> HListIntoIter<A> {
        HListIntoIter { list: self }
    }

    pub fn clear(&mut self) {
        while let Some(ptr) = self.pop_front() {
            drop(ptr)
        }
    }

    pub fn fast_clear(&mut self) {
        self.head = None;
    }

    pub fn take(&mut self) -> Self
    where
        A: Clone,
    {
        Self {
            head: self.head.take(),
            adapter: self.adapter.clone(),
            data: PhantomData,
        }
    }

    pub unsafe fn cursor_from_pointer(&self, ptr: *const A::Value) -> HListCursor<'_, A> {
        let link = self.adapter.get_link(ptr);
        HListCursor {
            inner: RawCursor {
                cur: NonNull::new_unchecked(link as _),
            },
            list: self,
        }
    }

    pub unsafe fn cursor_mut_from_pointer(
        &mut self,
        ptr: *const A::Value,
    ) -> HListCursorMut<'_, A> {
        let link = self.adapter.get_link(ptr);
        HListCursorMut {
            inner: RawCursor {
                cur: NonNull::new_unchecked(link as _),
            },
            list: self,
        }
    }

    fn front_raw(&self) -> Option<RawCursor<A>> {
        self.head.map(|head| RawCursor { cur: head })
    }

    unsafe fn mark_unlink(&mut self, link: NonNull<A::Link>) {
        if self.head == Some(link) {
            self.head = link.as_ref().get_next();
        }
    }
}
impl<A: ConstInit + Adapter<Link: ListLinkOps>> ConstInit for HList<A> {
    const INIT: Self = Self::new(A::INIT);
}

impl<A: Adapter<Link: ListLinkOps>> Drop for HList<A> {
    fn drop(&mut self) {
        self.clear()
    }
}

struct RawCursor<A: Adapter<Link: ListLinkOps>> {
    cur: NonNull<A::Link>,
}
impl<A: Adapter<Link: ListLinkOps>> Clone for RawCursor<A> {
    fn clone(&self) -> Self {
        *self
    }
}
impl<A: Adapter<Link: ListLinkOps>> Copy for RawCursor<A> {}

impl<A: Adapter<Link: ListLinkOps>> RawCursor<A> {
    fn next(self) -> Option<Self> {
        unsafe { self.cur.as_ref().get_next().map(|next| Self { cur: next }) }
    }
    fn prev(self) -> Option<Self> {
        unsafe { self.cur.as_ref().get_prev().map(|next| Self { cur: next }) }
    }

    fn move_next(&mut self) -> bool {
        self.next().map(|next| *self = next).is_some()
    }

    fn move_prev(&mut self) -> bool {
        self.prev().map(|prev| *self = prev).is_some()
    }

    fn get(self, adapter: &A) -> NonNull<A::Value> {
        unsafe { NonNull::new_unchecked(adapter.get_value(self.cur.as_ptr()) as _) }
    }

    unsafe fn unlink(self, adapter: &A) -> A::Pointer {
        let link = self.cur.as_ref();
        if let Some(prev) = link.get_prev() {
            let prev = prev.as_ref();
            prev.set_next(link.get_next());
        }
        if let Some(next) = link.get_next() {
            let next = next.as_ref();
            next.set_prev(link.get_prev());
        }
        link.set_next(None);
        link.set_prev(None);
        let meta = link.take_meta();
        link.release();
        let value = adapter.get_value(link);
        <A::Pointer as PointerOps>::from_raw(value, meta)
    }
}

pub struct HListCursor<'a, A: Adapter<Link: ListLinkOps>> {
    inner: RawCursor<A>,
    list: &'a HList<A>,
}
impl<'a, A: Adapter<Link: ListLinkOps>> Clone for HListCursor<'a, A> {
    fn clone(&self) -> Self {
        *self
    }
}
impl<'a, A: Adapter<Link: ListLinkOps>> Copy for HListCursor<'a, A> {}

impl<A: Adapter<Link: ListLinkOps>> HListCursor<'_, A> {
    pub fn next(self) -> Option<Self> {
        Some(Self {
            inner: self.inner.next()?,
            list: self.list,
        })
    }
    pub fn prev(self) -> Option<Self> {
        Some(Self {
            inner: self.inner.prev()?,
            list: self.list,
        })
    }
    pub fn move_next(&mut self) -> bool {
        self.inner.move_next()
    }
    pub fn move_prev(&mut self) -> bool {
        self.inner.move_prev()
    }
    pub fn get(&self) -> &A::Value {
        unsafe { self.inner.get(&self.list.adapter).as_ref() }
    }
}

pub struct HListCursorMut<'a, A: Adapter<Link: ListLinkOps>> {
    inner: RawCursor<A>,
    list: &'a mut HList<A>,
}
impl<A: Adapter<Link: ListLinkOps>> HListCursorMut<'_, A> {
    pub fn next(self) -> Option<Self> {
        Some(Self {
            inner: self.inner.next()?,
            list: self.list,
        })
    }
    pub fn prev(self) -> Option<Self> {
        Some(Self {
            inner: self.inner.prev()?,
            list: self.list,
        })
    }
    pub fn move_next(&mut self) -> bool {
        self.inner.move_next()
    }
    pub fn move_prev(&mut self) -> bool {
        self.inner.move_prev()
    }
    pub fn get(&self) -> &A::Value {
        unsafe { self.inner.get(&self.list.adapter).as_ref() }
    }
    pub fn get_mut(&mut self) -> &mut A::Value
    where
        A::Pointer: ExclusivePointerOps,
    {
        unsafe { self.inner.get(&self.list.adapter).as_mut() }
    }
    pub fn unlink(self) -> A::Pointer {
        let ptr = self.inner.cur;
        unsafe {
            self.list.mark_unlink(ptr);
            self.inner.unlink(&self.list.adapter)
        }
    }
}

pub struct HListIter<'a, A: Adapter<Link: ListLinkOps>> {
    cur: Option<RawCursor<A>>,
    list: &'a HList<A>,
}
impl<'a, A: Adapter<Link: ListLinkOps>> Iterator for HListIter<'a, A> {
    type Item = &'a A::Value;
    fn next(&mut self) -> Option<Self::Item> {
        self.cur.take().map(|cur| {
            let ptr = cur.get(&self.list.adapter);
            self.cur = cur.next();
            unsafe { ptr.as_ref() }
        })
    }
}
impl<'a, A: Adapter<Link: ListLinkOps>> DoubleEndedIterator for HListIter<'a, A> {
    fn next_back(&mut self) -> Option<Self::Item> {
        self.cur.take().map(|cur| {
            let ptr = cur.get(&self.list.adapter);
            self.cur = cur.prev();
            unsafe { ptr.as_ref() }
        })
    }
}
impl<'a, A: Adapter<Link: ListLinkOps>> IntoIterator for &'a HList<A> {
    type IntoIter = HListIter<'a, A>;
    type Item = &'a A::Value;
    fn into_iter(self) -> Self::IntoIter {
        self.iter()
    }
}

pub struct HListIterMut<'a, A: Adapter<Link: ListLinkOps, Pointer: ExclusivePointerOps>> {
    cur: Option<RawCursor<A>>,
    list: &'a mut HList<A>,
}
impl<'a, A: Adapter<Link: ListLinkOps, Pointer: ExclusivePointerOps>> Iterator
    for HListIterMut<'a, A>
{
    type Item = &'a mut A::Value;
    fn next(&mut self) -> Option<Self::Item> {
        self.cur.take().map(|cur| {
            let mut ptr = cur.get(&self.list.adapter);
            self.cur = cur.next();
            unsafe { ptr.as_mut() }
        })
    }
}
impl<'a, A: Adapter<Link: ListLinkOps, Pointer: ExclusivePointerOps>> DoubleEndedIterator
    for HListIterMut<'a, A>
{
    fn next_back(&mut self) -> Option<Self::Item> {
        self.cur.take().map(|cur| {
            let mut ptr = cur.get(&self.list.adapter);
            self.cur = cur.prev();
            unsafe { ptr.as_mut() }
        })
    }
}
impl<'a, A: Adapter<Link: ListLinkOps, Pointer: ExclusivePointerOps>> IntoIterator
    for &'a mut HList<A>
{
    type IntoIter = HListIterMut<'a, A>;
    type Item = &'a mut A::Value;
    fn into_iter(self) -> Self::IntoIter {
        self.iter_mut()
    }
}

pub struct HListIntoIter<A: Adapter<Link: ListLinkOps>> {
    list: HList<A>,
}
impl<A: Adapter<Link: ListLinkOps>> Iterator for HListIntoIter<A> {
    type Item = A::Pointer;
    fn next(&mut self) -> Option<Self::Item> {
        self.list.pop_front()
    }
}
impl<A: Adapter<Link: ListLinkOps>> IntoIterator for HList<A> {
    type IntoIter = HListIntoIter<A>;
    type Item = A::Pointer;
    fn into_iter(self) -> Self::IntoIter {
        Self::into_iter(self)
    }
}

pub(super) unsafe fn insert_before<L: ListLinkOps>(before_ptr: NonNull<L>, node_ptr: NonNull<L>) {
    let before = before_ptr.as_ref();
    let node = node_ptr.as_ref();
    node.set_next(Some(before_ptr));
    node.set_prev(before.get_prev());
    if let Some(prev) = before.get_prev() {
        prev.as_ref().set_next(Some(node_ptr));
    }
    before.set_prev(Some(node_ptr));
}

pub(super) unsafe fn insert_after<L: ListLinkOps>(after_ptr: NonNull<L>, node_ptr: NonNull<L>) {
    let after = after_ptr.as_ref();
    let node = node_ptr.as_ref();
    node.set_prev(Some(after_ptr));
    node.set_next(after.get_next());
    if let Some(next) = after.get_next() {
        next.as_ref().set_prev(Some(node_ptr));
    }
    after.set_next(Some(node_ptr));
}

#[cfg(test)]
mod tests {
    use std::prelude::v1::*;

    use alloc::sync::Arc;

    use super::*;

    struct A {
        value: u32,
        link: AtomicLink<()>,
    }
    impl A {
        const fn new(value: u32) -> Self {
            Self {
                value,
                link: AtomicLink::new(),
            }
        }
    }

    crate::intrusive_adapter!(struct AAdapter<'a> = &'a A : A {link: AtomicLink<()> });
    crate::intrusive_adapter!(struct AAdapterMut<'a> = &'a mut A : A { link: AtomicLink<()> });
    crate::intrusive_adapter!(struct AAdapterArc<'a> = Arc<A> : A { link: AtomicLink<()> });

    fn with_hlist<R>(f: impl FnOnce(&mut HList<AAdapterMut>) -> R) -> R {
        let mut a0 = A::new(0);
        let mut a1 = A::new(1);
        let mut a2 = A::new(2);
        let mut list = HList::new(AAdapterMut::new());
        list.push_front(&mut a0);
        list.push_front(&mut a1);
        list.push_front(&mut a2);
        f(&mut list)
    }

    #[test]
    fn hlist_push_front() {
        with_hlist(|list| {
            let v: Vec<_> = list.iter().map(|a| a.value).collect();
            assert_eq!(v, [2, 1, 0]);
        });
    }

    #[test]
    fn hlist_push_front_dup() {
        let a0 = A::new(0);
        let mut list = HList::new(AAdapter::new());
        assert!(list.try_push_front(&a0).is_ok());
        assert!(list.try_push_front(&a0).is_err());
    }

    #[test]
    fn hlist_pop_front() {
        with_hlist(|list| {
            assert_eq!(list.pop_front().map(|a| a.value), Some(2));
            assert_eq!(list.pop_front().map(|a| a.value), Some(1));
            assert_eq!(list.pop_front().map(|a| a.value), Some(0));
            assert_eq!(list.pop_front().map(|a| a.value), None);
        });
    }

    #[test]
    fn hlist_iter() {
        with_hlist(|list| {
            let mut iter = list.iter();
            assert_eq!(iter.next().map(|a| a.value), Some(2));
            assert_eq!(iter.next().map(|a| a.value), Some(1));
            assert_eq!(iter.next().map(|a| a.value), Some(0));
            assert_eq!(iter.next().map(|a| a.value), None);
        });
    }

    #[test]
    fn hlist_iter_mut() {
        with_hlist(|list| {
            for a in list.iter_mut() {
                a.value += 1;
            }
            let v: Vec<_> = list.iter().map(|a| a.value).collect();
            assert_eq!(v, [3, 2, 1]);
        });
    }

    #[test]
    fn hlist_into_iter() {
        let mut a0 = A::new(0);
        let mut a1 = A::new(1);
        let mut a2 = A::new(2);
        let mut list = HList::new(AAdapterMut::new());
        list.push_front(&mut a0);
        list.push_front(&mut a1);
        list.push_front(&mut a2);
        for a in list {
            a.value += 1;
        }
        assert_eq!(a0.value, 1);
        assert_eq!(a1.value, 2);
        assert_eq!(a2.value, 3);
    }

    #[test]
    fn hlist_cursor() {
        with_hlist(|list| {
            let mut cur = list.front();
            assert_eq!(cur.as_ref().map(|c| c.get().value), Some(2));
            cur = cur.unwrap().next();
            assert_eq!(cur.as_ref().map(|c| c.get().value), Some(1));
            cur = cur.unwrap().next();
            assert_eq!(cur.as_ref().map(|c| c.get().value), Some(0));
            cur = cur.unwrap().next();
            assert_eq!(cur.as_ref().map(|c| c.get().value), None);
        });
    }

    #[test]
    fn hlist_cursor_mut() {
        with_hlist(|list| {
            let mut cur = list.front_mut();
            assert!(cur.is_some());
            cur.as_mut().unwrap().get_mut().value += 1;
            cur = cur.unwrap().next();
            assert!(cur.is_some());
            cur.as_mut().unwrap().get_mut().value += 1;
            cur = cur.unwrap().next();
            assert!(cur.is_some());
            cur.as_mut().unwrap().get_mut().value += 1;
            cur = cur.unwrap().next();
            assert!(cur.is_none());
            let v: Vec<_> = list.iter().map(|a| a.value).collect();
            assert_eq!(v, [3, 2, 1]);
        });
    }

    #[test]
    fn hlist_cursor_unlink() {
        with_hlist(|list| {
            let cur = list.front_mut();
            let cur = cur.unwrap().next();
            cur.unwrap().unlink();
            let v: Vec<_> = list.iter().map(|a| a.value).collect();
            assert_eq!(v, [2, 0]);
        });
    }

    #[test]
    fn hlist_clear() {
        let a0 = A::new(0);
        let a1 = A::new(1);
        let a2 = A::new(2);
        let mut list = HList::new(AAdapter::new());
        list.push_front(&a0);
        list.push_front(&a1);
        list.push_front(&a2);
        list.clear();
        assert!(!a0.link.is_linked());
        assert!(!a1.link.is_linked());
        assert!(!a2.link.is_linked());
    }

    #[test]
    fn hlist_fast_clear() {
        let a0 = A::new(0);
        let a1 = A::new(1);
        let a2 = A::new(2);
        let mut list = HList::new(AAdapter::new());
        list.push_front(&a0);
        list.push_front(&a1);
        list.push_front(&a2);
        list.fast_clear();
        assert!(a0.link.is_linked());
        assert!(a1.link.is_linked());
        assert!(a2.link.is_linked());
    }

    #[test]
    fn hlist_drop() {
        let a0 = Arc::new(A::new(0));
        let a1 = Arc::new(A::new(0));
        let a2 = Arc::new(A::new(0));
        let mut list = HList::new(AAdapterArc::new());
        list.push_front(a0.clone());
        list.push_front(a1.clone());
        list.push_front(a2.clone());
        assert_eq!(Arc::strong_count(&a0), 2);
        assert_eq!(Arc::strong_count(&a1), 2);
        assert_eq!(Arc::strong_count(&a2), 2);
        drop(list);
        assert_eq!(Arc::strong_count(&a0), 1);
        assert_eq!(Arc::strong_count(&a1), 1);
        assert_eq!(Arc::strong_count(&a2), 1);
    }

    #[test]
    fn link_force_unlink() {
        let a0 = A::new(0);
        let mut list = HList::new(AAdapter::new());
        list.push_front(&a0);
        list.fast_clear();
        assert!(a0.link.is_linked());
        unsafe { a0.link.force_unlink() };
        assert!(!a0.link.is_linked());
    }
}
