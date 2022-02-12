use core::borrow::Borrow;
use core::hash::{BuildHasher, Hash};
use core::marker::PhantomData;

use intrusive_collections::{
    linked_list, Adapter, ExclusivePointerOps, KeyAdapter, LinkOps, PointerOps,
};
pub use linked_list::{AtomicLink, AtomicLinkOps, LinkedListOps as HashTableOps};
pub use siphasher::sip::SipHasher as DefaultHasher;

use crate::init::ConstInit;

pub struct DefaultState;

impl BuildHasher for DefaultState {
    type Hasher = DefaultHasher;

    fn build_hasher(&self) -> Self::Hasher {
        // Random number
        const DEFAULT_KEY: [u8; 16] = [
            0x61, 0x0d, 0xc7, 0xce, 0x1e, 0xc3, 0x64, 0x93, 0x23, 0x16, 0xea, 0xbc, 0x05, 0x87,
            0x3f, 0x9e,
        ];
        DefaultHasher::new_with_key(&DEFAULT_KEY)
    }
}
impl ConstInit for DefaultState {
    const INIT: Self = Self;
}

pub trait Buckets<T>: AsMut<[T]> + Sized {
    fn reserve(&mut self, min_capacity_order: u8) -> Option<Self>;
    fn shrink_to(&mut self, min_capacity_order: u8) -> Option<Self>;
    fn capacity_order(&self) -> u8;
}

pub struct HashTable<A: Adapter<LinkOps: HashTableOps>, S: BuildHasher, const BUCKETS: usize> {
    buckets: [Option<<A::LinkOps as LinkOps>::LinkPtr>; BUCKETS],
    adapter: A,
    state: S,
    data: PhantomData<<A::PointerOps as PointerOps>::Pointer>,
}
impl<A: Adapter<LinkOps: HashTableOps>, S: BuildHasher, const BUCKETS: usize>
    HashTable<A, S, BUCKETS>
{
    pub const fn with_state(adapter: A, state: S) -> Self {
        Self {
            buckets: [None; BUCKETS],
            adapter,
            state,
            data: PhantomData,
        }
    }

    pub const fn new(adapter: A) -> Self
    where
        S: ConstInit,
    {
        Self::with_state(adapter, S::INIT)
    }

    pub fn cursor(&self) -> Cursor<'_, A, S, BUCKETS> {
        Cursor {
            cursor: self.cursor_raw(),
            table: self,
        }
    }

    pub fn cursor_mut(&mut self) -> CursorMut<'_, A, S, BUCKETS> {
        CursorMut {
            cursor: self.cursor_raw(),
            table: self,
        }
    }

    pub fn front(&self) -> Cursor<'_, A, S, BUCKETS> {
        Cursor {
            cursor: self.front_raw(),
            table: self,
        }
    }

    pub fn front_mut(&mut self) -> CursorMut<'_, A, S, BUCKETS> {
        CursorMut {
            cursor: self.front_raw(),
            table: self,
        }
    }

    pub fn iter(&self) -> Iter<'_, A, S, BUCKETS> {
        Iter {
            cursor: self.front_raw(),
            table: self,
        }
    }

    pub unsafe fn iter_mut(&mut self) -> IterMut<'_, A, S, BUCKETS> {
        IterMut {
            cursor: self.front_raw(),
            table: self,
        }
    }

    fn cursor_raw(&self) -> RawCursor<A> {
        RawCursor {
            bucket: 0,
            cur: None,
        }
    }

    fn front_raw(&self) -> RawCursor<A> {
        self.buckets
            .iter()
            .enumerate()
            .find_map(|(bucket, cur)| {
                cur.map(|cur| RawCursor {
                    bucket,
                    cur: Some(cur),
                })
            })
            .unwrap_or_else(|| self.cursor_raw())
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct LinkAlreadyAcquiredError;

impl<'a, A: KeyAdapter<'a, LinkOps: HashTableOps>, S: BuildHasher, const BUCKETS: usize>
    HashTable<A, S, BUCKETS>
where
    <A::PointerOps as PointerOps>::Value: 'a,
{
    pub fn try_insert(
        &mut self,
        ptr: <A::PointerOps as PointerOps>::Pointer,
    ) -> Result<(), LinkAlreadyAcquiredError>
    where
        A::Key: Hash,
    {
        unsafe {
            let value = self.adapter.pointer_ops().into_raw(ptr);
            let bucket = self.bucket_for(&self.adapter.get_key(&*value));
            let link = self.adapter.get_link(value);
            if self.adapter.link_ops_mut().acquire_link(link) {
                insert_in_bucket(&mut self.adapter, &mut self.buckets[bucket], link);
                Ok(())
            } else {
                Err(LinkAlreadyAcquiredError)
            }
        }
    }

    pub fn insert(&mut self, ptr: <A::PointerOps as PointerOps>::Pointer)
    where
        A::Key: Hash,
    {
        self.try_insert(ptr).expect("Already linked")
    }

    pub fn find<Q>(&self, key: &Q) -> Cursor<'_, A, S, BUCKETS>
    where
        Q: Borrow<A::Key>,
        A::Key: Hash + Eq,
    {
        Cursor {
            cursor: self.find_raw(key.borrow()),
            table: self,
        }
    }

    pub fn find_mut<Q>(&mut self, key: &Q) -> CursorMut<'_, A, S, BUCKETS>
    where
        Q: Borrow<A::Key>,
        A::Key: Hash + Eq,
    {
        CursorMut {
            cursor: self.find_raw(key.borrow()),
            table: self,
        }
    }

    fn find_raw(&self, key: &A::Key) -> RawCursor<A>
    where
        A::Key: Hash + Eq,
    {
        let bucket = self.bucket_for(key);
        let mut cur_opt = self.buckets[bucket];
        while let Some(cur) = cur_opt {
            let value = unsafe { &*self.adapter.get_value(cur) };
            if &self.adapter.get_key(value) == key {
                return RawCursor {
                    bucket: bucket,
                    cur: Some(cur),
                };
            }
            cur_opt = unsafe { self.adapter.link_ops().next(cur) };
        }
        self.cursor_raw()
    }

    fn bucket_for(&self, key: &A::Key) -> usize
    where
        A::Key: Hash,
    {
        self.state.hash_one(key) as usize % BUCKETS
    }
}

unsafe fn insert_in_bucket<A: Adapter<LinkOps: HashTableOps>>(
    adapter: &mut A,
    bucket: &mut Option<<A::LinkOps as LinkOps>::LinkPtr>,
    node: <A::LinkOps as LinkOps>::LinkPtr,
) {
    let ops = adapter.link_ops_mut();
    if let Some(head) = bucket {
        ops.set_next(node, Some(*head));
        ops.set_prev(*head, Some(node));
        *head = node;
    } else {
        *bucket = Some(node);
    }
}

struct RawCursor<A: Adapter<LinkOps: HashTableOps>> {
    cur: Option<<A::LinkOps as LinkOps>::LinkPtr>,
    bucket: usize,
}
impl<A: Adapter<LinkOps: HashTableOps>> Clone for RawCursor<A> {
    fn clone(&self) -> Self {
        *self
    }
}
impl<A: Adapter<LinkOps: HashTableOps>> Copy for RawCursor<A> {}

impl<A: Adapter<LinkOps: HashTableOps>> RawCursor<A> {
    fn move_next(&mut self, adapter: &A, buckets: &[Option<<A::LinkOps as LinkOps>::LinkPtr>]) {
        if let Some(cur) = self.cur {
            self.cur = unsafe { adapter.link_ops().next(cur) }
        } else {
            for i in (self.bucket + 1)..buckets.len() {
                if let Some(head) = buckets[i] {
                    self.cur = Some(head);
                    self.bucket = i;
                    return;
                }
            }
        }
    }

    fn move_prev(&mut self, adapter: &A, buckets: &[Option<<A::LinkOps as LinkOps>::LinkPtr>]) {
        if let Some(cur) = self.cur {
            self.cur = unsafe { adapter.link_ops().prev(cur) }
        } else {
            for i in (0..self.bucket).rev() {
                if let Some(head) = buckets[i] {
                    self.cur = Some(head);
                    self.bucket = i;
                    return;
                }
            }
        }
    }

    fn is_valid(&self) -> bool {
        self.cur.is_some()
    }

    unsafe fn get<'a>(self, adapter: &A) -> Option<&'a <A::PointerOps as PointerOps>::Value> {
        self.cur.map(|link| unsafe { &*adapter.get_value(link) })
    }

    unsafe fn get_mut<'a>(
        self,
        adapter: &A,
    ) -> Option<&'a mut <A::PointerOps as PointerOps>::Value> {
        self.cur
            .map(|link| &mut *(adapter.get_value(link) as *mut _))
    }

    unsafe fn unlink(
        self,
        adapter: &mut A,
        buckets: &mut [Option<<A::LinkOps as LinkOps>::LinkPtr>],
    ) -> Option<<A::LinkOps as LinkOps>::LinkPtr> {
        let ops = adapter.link_ops_mut();
        self.cur.map(|cur| {
            if let Some(next) = ops.next(cur) {
                ops.set_prev(next, ops.prev(cur));
            }
            if let Some(prev) = ops.prev(cur) {
                ops.set_next(prev, ops.next(cur));
            }
            if buckets[self.bucket] == Some(cur) {
                buckets[self.bucket] = ops.next(cur);
            }
            ops.set_next(cur, None);
            ops.set_prev(cur, None);
            ops.release_link(cur);
            cur
        })
    }
}

pub struct Cursor<'a, A: Adapter<LinkOps: HashTableOps>, S: BuildHasher, const BUCKETS: usize> {
    cursor: RawCursor<A>,
    table: &'a HashTable<A, S, BUCKETS>,
}
impl<'a, A: Adapter<LinkOps: HashTableOps>, S: BuildHasher, const BUCKETS: usize> Clone
    for Cursor<'a, A, S, BUCKETS>
{
    fn clone(&self) -> Self {
        *self
    }
}
impl<'a, A: Adapter<LinkOps: HashTableOps>, S: BuildHasher, const BUCKETS: usize> Copy
    for Cursor<'a, A, S, BUCKETS>
{
}

impl<'a, A: Adapter<LinkOps: HashTableOps>, S: BuildHasher, const BUCKETS: usize>
    Cursor<'a, A, S, BUCKETS>
{
    pub fn move_next(&mut self) {
        self.cursor
            .move_next(&self.table.adapter, &self.table.buckets)
    }

    pub fn move_prev(&mut self) {
        self.cursor
            .move_prev(&self.table.adapter, &self.table.buckets)
    }

    pub fn is_valid(&self) -> bool {
        self.cursor.is_valid()
    }

    pub fn get(&self) -> Option<&<A::PointerOps as PointerOps>::Value> {
        unsafe { self.cursor.get(&self.table.adapter) }
    }
}

pub struct CursorMut<'a, A: Adapter<LinkOps: HashTableOps>, S: BuildHasher, const BUCKETS: usize> {
    cursor: RawCursor<A>,
    table: &'a mut HashTable<A, S, BUCKETS>,
}

impl<'a, A: Adapter<LinkOps: HashTableOps>, S: BuildHasher, const BUCKETS: usize>
    CursorMut<'a, A, S, BUCKETS>
{
    pub fn move_next(&mut self) {
        self.cursor
            .move_next(&self.table.adapter, &self.table.buckets)
    }

    pub fn move_prev(&mut self) {
        self.cursor
            .move_prev(&self.table.adapter, &self.table.buckets)
    }

    pub fn is_valid(&self) -> bool {
        self.cursor.is_valid()
    }

    pub fn get(&self) -> Option<&<A::PointerOps as PointerOps>::Value> {
        unsafe { self.cursor.get(&self.table.adapter) }
    }

    pub unsafe fn get_mut(&mut self) -> Option<&mut <A::PointerOps as PointerOps>::Value>
    where
        A::PointerOps: ExclusivePointerOps,
    {
        self.cursor.get_mut(&self.table.adapter)
    }

    pub fn unlink(self) -> Option<<A::PointerOps as PointerOps>::Pointer> {
        unsafe {
            self.cursor
                .unlink(&mut self.table.adapter, &mut self.table.buckets)
                .map(move |link| {
                    self.table
                        .adapter
                        .pointer_ops()
                        .from_raw(self.table.adapter.get_value(link))
                })
        }
    }
}

pub struct Iter<'a, A: Adapter<LinkOps: HashTableOps>, S: BuildHasher, const BUCKETS: usize> {
    cursor: RawCursor<A>,
    table: &'a HashTable<A, S, BUCKETS>,
}

impl<'a, A: Adapter<LinkOps: HashTableOps>, S: BuildHasher, const BUCKETS: usize> Iterator
    for Iter<'a, A, S, BUCKETS>
{
    type Item = &'a <A::PointerOps as PointerOps>::Value;
    fn next(&mut self) -> Option<Self::Item> {
        unsafe { self.cursor.get(&self.table.adapter) }.map(move |value| {
            self.cursor
                .move_next(&self.table.adapter, &self.table.buckets);
            value
        })
    }
}

impl<'a, A: Adapter<LinkOps: HashTableOps>, S: BuildHasher, const BUCKETS: usize> IntoIterator
    for &'a HashTable<A, S, BUCKETS>
{
    type IntoIter = Iter<'a, A, S, BUCKETS>;
    type Item = &'a <A::PointerOps as PointerOps>::Value;
    fn into_iter(self) -> Self::IntoIter {
        self.iter()
    }
}

pub struct IterMut<'a, A: Adapter<LinkOps: HashTableOps>, S: BuildHasher, const BUCKETS: usize> {
    cursor: RawCursor<A>,
    table: &'a mut HashTable<A, S, BUCKETS>,
}

impl<'a, A: Adapter<LinkOps: HashTableOps>, S: BuildHasher, const BUCKETS: usize> Iterator
    for IterMut<'a, A, S, BUCKETS>
{
    type Item = &'a mut <A::PointerOps as PointerOps>::Value;
    fn next(&mut self) -> Option<Self::Item> {
        unsafe { self.cursor.get_mut(&self.table.adapter) }.map(move |value| {
            self.cursor
                .move_next(&self.table.adapter, &self.table.buckets);
            value
        })
    }
}

#[cfg(test)]
mod tests {
    use std::prelude::v1::*;

    use alloc::sync::Arc;
    use intrusive_collections::intrusive_adapter;

    use super::*;
    struct A {
        link: AtomicLink,
        key: u32,
        value: u32,
    }

    impl A {
        fn new(value: u32) -> Self {
            Self {
                link: AtomicLink::new(),
                key: value,
                value,
            }
        }

        fn boxed(value: u32) -> Box<Self> {
            Box::new(Self::new(value))
        }
    }

    intrusive_adapter!(AdBox = Box<A> : A { link: AtomicLink });
    intrusive_adapter!(AdArc = Arc<A> : A { link: AtomicLink });

    type HT<A> = HashTable<A, DefaultState, 2>;

    #[test]
    fn insert() {
        let mut table = HT::new(AdBox::new());
        let a0 = A::boxed(0);
        let a1 = A::boxed(1);
        let a2 = A::boxed(2);
        table.insert(a0);
        table.insert(a1);
        table.insert(a2);
    }
}
