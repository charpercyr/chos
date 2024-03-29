use core::borrow::Borrow;
use core::fmt;
use core::hash::{BuildHasher, Hash};
use core::marker::PhantomData;

use intrusive_collections::{
    linked_list, Adapter, ExclusivePointerOps, KeyAdapter, LinkOps, PointerOps,
};
pub use linked_list::{AtomicLink, AtomicLinkOps, LinkedListOps as HashTableOps};
pub use siphasher::sip::SipHasher as DefaultHasher;

use crate::array::{const_len_of_val, Array};
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

pub trait BucketSize {
    type Array<T: ConstInit>: Array<T> + ConstInit;
}
pub mod sizes {
    macro_rules! bucket_size
    {
        ($($o:expr),* $(,)?) => {
            paste::item! {
                $(
                    pub struct [<O $o>](!);
                    impl super::BucketSize for [<O $o>] {
                        type Array<T: super::ConstInit> = [T; 1 << $o];
                    }
                )*
            }
        };
    }
    bucket_size!(0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16);
}

pub struct HashTable<
    A: Adapter<LinkOps: HashTableOps>,
    B: BucketSize,
    S: BuildHasher = DefaultState,
> {
    buckets: B::Array<Option<<A::LinkOps as LinkOps>::LinkPtr>>,
    adapter: A,
    state: S,
    data: PhantomData<<A::PointerOps as PointerOps>::Pointer>,
}
impl<A: Adapter<LinkOps: HashTableOps>, B: BucketSize, S: BuildHasher> HashTable<A, B, S> {
    pub const fn with_state(adapter: A, state: S) -> Self {
        Self {
            buckets: ConstInit::INIT,
            adapter,
            state,
            data: PhantomData,
        }
    }

    pub fn cursor(&self) -> Cursor<'_, A, B, S> {
        Cursor {
            cursor: self.cursor_raw(),
            table: self,
        }
    }

    pub fn cursor_mut(&mut self) -> CursorMut<'_, A, B, S> {
        CursorMut {
            cursor: self.cursor_raw(),
            table: self,
        }
    }

    pub fn front(&self) -> Cursor<'_, A, B, S> {
        Cursor {
            cursor: self.front_raw(),
            table: self,
        }
    }

    pub fn front_mut(&mut self) -> CursorMut<'_, A, B, S> {
        CursorMut {
            cursor: self.front_raw(),
            table: self,
        }
    }

    pub fn iter(&self) -> Iter<'_, A, B, S> {
        Iter {
            cursor: self.front_raw(),
            table: self,
        }
    }

    pub unsafe fn iter_mut(&mut self) -> IterMut<'_, A, B, S> {
        IterMut {
            cursor: self.front_raw(),
            table: self,
        }
    }

    pub fn fast_clear(&mut self) {
        for b in self.buckets.as_mut() {
            *b = None;
        }
    }

    pub fn clear(&mut self) {
        for b in self.buckets.as_mut() {
            let mut cur = *b;
            while let Some(node) = cur {
                unsafe {
                    let link_ops = self.adapter.link_ops_mut();
                    cur = link_ops.next(node);
                    link_ops.set_prev(node, None);
                    link_ops.set_next(node, None);
                    link_ops.release_link(node);
                    drop(
                        self.adapter
                            .pointer_ops()
                            .from_raw(self.adapter.get_value(node)),
                    );
                }
            }
            *b = None;
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
            .as_ref()
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
impl<A: Adapter<LinkOps: HashTableOps>, B: BucketSize, S: BuildHasher> fmt::Debug
    for HashTable<A, B, S>
where
    <A::PointerOps as PointerOps>::Value: fmt::Debug,
{
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.debug_set().entries(self.iter()).finish()
    }
}

unsafe impl<A: Adapter<LinkOps: HashTableOps> + Send, B: BucketSize, S: BuildHasher + Send> Send
    for HashTable<A, B, S>
where
    <A::PointerOps as PointerOps>::Pointer: Send,
{
}
unsafe impl<A: Adapter<LinkOps: HashTableOps> + Sync, B: BucketSize, S: BuildHasher + Sync> Sync
    for HashTable<A, B, S>
where
    <A::PointerOps as PointerOps>::Pointer: Sync,
{
}

impl<A: Adapter<LinkOps: HashTableOps> + ConstInit, B: BucketSize, S: BuildHasher + ConstInit>
    ConstInit for HashTable<A, B, S>
{
    const INIT: Self = Self::with_state(ConstInit::INIT, ConstInit::INIT);
}

impl<A: Adapter<LinkOps: HashTableOps>, B: BucketSize> HashTable<A, B> {
    pub const fn new(adapter: A) -> Self {
        Self::with_state(adapter, DefaultState::INIT)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct LinkAlreadyAcquiredError;

impl<'a, A: KeyAdapter<'a, LinkOps: HashTableOps>, B: BucketSize, S: BuildHasher> HashTable<A, B, S>
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
                insert_in_bucket(&mut self.adapter, &mut self.buckets.as_mut()[bucket], link);
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

    pub fn find<Q>(&self, key: &Q) -> Cursor<'_, A, B, S>
    where
        Q: Borrow<A::Key>,
        A::Key: Hash + Eq,
    {
        Cursor {
            cursor: self.find_raw(key.borrow()),
            table: self,
        }
    }

    pub fn find_mut<Q>(&mut self, key: &Q) -> CursorMut<'_, A, B, S>
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
        let mut cur_opt = self.buckets.as_ref()[bucket];
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
        self.state.hash_one(key) as usize % const_len_of_val(&self.buckets)
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
        ops.set_prev(node, None);
        ops.set_prev(*head, Some(node));
        *head = node;
    } else {
        *bucket = Some(node);
        ops.set_next(node, None);
        ops.set_prev(node, None);
    }
}

impl<A: Adapter<LinkOps: HashTableOps>, B: BucketSize, S: BuildHasher> Drop for HashTable<A, B, S> {
    fn drop(&mut self) {
        self.clear()
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
            if let Some(next) = unsafe { adapter.link_ops().next(cur) } {
                self.cur = Some(next);
                return;
            }
        }
        for i in (self.bucket + 1)..buckets.len() {
            if let Some(head) = buckets[i] {
                self.cur = Some(head);
                self.bucket = i;
                return;
            }
        }
        self.cur = None;
    }

    fn is_valid(&self) -> bool {
        self.cur.is_some()
    }

    unsafe fn get<'a>(self, adapter: &A) -> Option<&'a <A::PointerOps as PointerOps>::Value> {
        self.cur.map(|link| &*adapter.get_value(link))
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

pub struct Cursor<'a, A: Adapter<LinkOps: HashTableOps>, B: BucketSize, S: BuildHasher> {
    cursor: RawCursor<A>,
    table: &'a HashTable<A, B, S>,
}
impl<'a, A: Adapter<LinkOps: HashTableOps>, B: BucketSize, S: BuildHasher> Clone
    for Cursor<'a, A, B, S>
{
    fn clone(&self) -> Self {
        *self
    }
}
impl<'a, A: Adapter<LinkOps: HashTableOps>, B: BucketSize, S: BuildHasher> Copy
    for Cursor<'a, A, B, S>
{
}

impl<'a, A: Adapter<LinkOps: HashTableOps>, B: BucketSize, S: BuildHasher> Cursor<'a, A, B, S> {
    pub fn move_next(&mut self) {
        self.cursor
            .move_next(&self.table.adapter, self.table.buckets.as_ref())
    }

    pub fn is_valid(&self) -> bool {
        self.cursor.is_valid()
    }

    pub fn get(&self) -> Option<&<A::PointerOps as PointerOps>::Value> {
        unsafe { self.cursor.get(&self.table.adapter) }
    }
}

pub struct CursorMut<'a, A: Adapter<LinkOps: HashTableOps>, B: BucketSize, S: BuildHasher> {
    cursor: RawCursor<A>,
    table: &'a mut HashTable<A, B, S>,
}

impl<'a, A: Adapter<LinkOps: HashTableOps>, B: BucketSize, S: BuildHasher> CursorMut<'a, A, B, S> {
    pub fn move_next(&mut self) {
        self.cursor
            .move_next(&self.table.adapter, self.table.buckets.as_ref())
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
                .unlink(&mut self.table.adapter, self.table.buckets.as_mut())
                .map(move |link| {
                    self.table
                        .adapter
                        .pointer_ops()
                        .from_raw(self.table.adapter.get_value(link))
                })
        }
    }
}

pub struct Iter<'a, A: Adapter<LinkOps: HashTableOps>, B: BucketSize, S: BuildHasher> {
    cursor: RawCursor<A>,
    table: &'a HashTable<A, B, S>,
}

impl<'a, A: Adapter<LinkOps: HashTableOps>, B: BucketSize, S: BuildHasher> Iterator
    for Iter<'a, A, B, S>
{
    type Item = &'a <A::PointerOps as PointerOps>::Value;
    fn next(&mut self) -> Option<Self::Item> {
        unsafe { self.cursor.get(&self.table.adapter) }.map(move |value| {
            self.cursor
                .move_next(&self.table.adapter, self.table.buckets.as_ref());
            value
        })
    }
}

impl<'a, A: Adapter<LinkOps: HashTableOps>, B: BucketSize, S: BuildHasher> IntoIterator
    for &'a HashTable<A, B, S>
{
    type IntoIter = Iter<'a, A, B, S>;
    type Item = &'a <A::PointerOps as PointerOps>::Value;
    fn into_iter(self) -> Self::IntoIter {
        self.iter()
    }
}

pub struct IterMut<'a, A: Adapter<LinkOps: HashTableOps>, B: BucketSize, S: BuildHasher> {
    cursor: RawCursor<A>,
    table: &'a mut HashTable<A, B, S>,
}

impl<'a, A: Adapter<LinkOps: HashTableOps>, B: BucketSize, S: BuildHasher> Iterator
    for IterMut<'a, A, B, S>
{
    type Item = &'a mut <A::PointerOps as PointerOps>::Value;
    fn next(&mut self) -> Option<Self::Item> {
        unsafe { self.cursor.get_mut(&self.table.adapter) }.map(move |value| {
            self.cursor
                .move_next(&self.table.adapter, self.table.buckets.as_ref());
            value
        })
    }
}

#[cfg(test)]
mod tests {
    // For some reason, there is an undefined reference to AsMut<[T; N]>::as_mut & AsRef<[T; N]>::as_ref
    // use alloc::boxed::Box;
    // use alloc::sync::Arc;
    // use alloc::vec::Vec;

    // use intrusive_collections::intrusive_adapter;

    // use super::*;
    // struct A {
    //     link: AtomicLink,
    //     key: u32,
    //     value: u32,
    // }

    // impl A {
    //     fn new(value: u32) -> Self {
    //         Self {
    //             link: AtomicLink::new(),
    //             key: value,
    //             value,
    //         }
    //     }

    //     fn boxed(value: u32) -> Box<Self> {
    //         Box::new(Self::new(value))
    //     }

    //     fn arc(value: u32) -> Arc<Self> {
    //         Arc::new(Self::new(value))
    //     }
    // }

    // intrusive_adapter!(AdBox = Box<A> : A { link: AtomicLink });
    // intrusive_adapter!(AdArc = Arc<A> : A { link: AtomicLink });

    // impl<'a> KeyAdapter<'a> for AdBox {
    //     type Key = u32;
    //     fn get_key(&self, a: &'a A) -> u32 {
    //         a.key
    //     }
    // }

    // impl<'a> KeyAdapter<'a> for AdArc {
    //     type Key = u32;
    //     fn get_key(&self, a: &'a A) -> u32 {
    //         a.key
    //     }
    // }

    // type HT<A> = HashTable<A, sizes::O4>;

    // #[test]
    // fn insert() {
    //     let mut table = HT::new(AdBox::new());
    //     let a0 = A::boxed(0);
    //     let a1 = A::boxed(1);
    //     let a2 = A::boxed(2);
    //     table.insert(a0);
    //     table.insert(a1);
    //     table.insert(a2);

    //     let mut values: Vec<_> = table.iter().map(|a| a.value).collect();
    //     values.sort_unstable();

    //     assert_eq!(values, [0, 1, 2]);
    // }

    // #[test]
    // fn unlink() {
    //     let mut table = HT::new(AdBox::new());
    //     let a0 = A::boxed(0);
    //     let a1 = A::boxed(1);
    //     let a2 = A::boxed(2);
    //     table.insert(a0);
    //     table.insert(a1);
    //     table.insert(a2);

    //     assert_eq!(table.find_mut(&1).unlink().map(|a| a.value), Some(1));
    //     assert_eq!(table.find_mut(&1).unlink().map(|a| a.value), None);

    //     let mut values: Vec<_> = table.iter().map(|a| a.value).collect();
    //     values.sort_unstable();

    //     assert_eq!(values, [0, 2]);
    // }

    // #[test]
    // fn iter_mut() {
    //     let mut table = HT::new(AdBox::new());
    //     let a0 = A::boxed(0);
    //     let a1 = A::boxed(1);
    //     let a2 = A::boxed(2);
    //     table.insert(a0);
    //     table.insert(a1);
    //     table.insert(a2);

    //     for i in unsafe { table.iter_mut() } {
    //         i.value += 1;
    //     }

    //     let mut values: Vec<_> = table.iter().map(|a| a.value).collect();
    //     values.sort_unstable();

    //     assert_eq!(values, [1, 2, 3]);
    // }

    // #[test]
    // fn fast_clear() {
    //     let mut table = HT::new(AdArc::new());
    //     let a0 = A::arc(0);
    //     let a1 = A::arc(1);
    //     let a2 = A::arc(2);
    //     table.insert(a0.clone());
    //     table.insert(a1.clone());
    //     table.insert(a2.clone());

    //     assert_eq!(Arc::strong_count(&a0), 2);
    //     assert_eq!(Arc::strong_count(&a1), 2);
    //     assert_eq!(Arc::strong_count(&a2), 2);

    //     table.fast_clear();

    //     assert_eq!(Arc::strong_count(&a0), 2);
    //     assert_eq!(Arc::strong_count(&a1), 2);
    //     assert_eq!(Arc::strong_count(&a2), 2);

    //     assert!(table.iter().next().is_none());
    // }

    // #[test]
    // fn clear() {
    //     let mut table = HT::new(AdArc::new());
    //     let a0 = A::arc(0);
    //     let a1 = A::arc(1);
    //     let a2 = A::arc(2);
    //     table.insert(a0.clone());
    //     table.insert(a1.clone());
    //     table.insert(a2.clone());

    //     assert_eq!(Arc::strong_count(&a0), 2);
    //     assert_eq!(Arc::strong_count(&a1), 2);
    //     assert_eq!(Arc::strong_count(&a2), 2);

    //     table.clear();

    //     assert_eq!(Arc::strong_count(&a0), 1);
    //     assert_eq!(Arc::strong_count(&a1), 1);
    //     assert_eq!(Arc::strong_count(&a2), 1);

    //     assert!(table.iter().next().is_none());
    // }

    // #[test]
    // fn drop() {
    //     let mut table = HT::new(AdArc::new());
    //     let a0 = A::arc(0);
    //     let a1 = A::arc(1);
    //     let a2 = A::arc(2);
    //     table.insert(a0.clone());
    //     table.insert(a1.clone());
    //     table.insert(a2.clone());

    //     assert_eq!(Arc::strong_count(&a0), 2);
    //     assert_eq!(Arc::strong_count(&a1), 2);
    //     assert_eq!(Arc::strong_count(&a2), 2);

    //     core::mem::drop(table);

    //     assert_eq!(Arc::strong_count(&a0), 1);
    //     assert_eq!(Arc::strong_count(&a1), 1);
    //     assert_eq!(Arc::strong_count(&a2), 1);
    // }
}
