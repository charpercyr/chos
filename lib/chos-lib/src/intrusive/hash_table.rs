use core::borrow::Borrow;
use core::hash::{BuildHasher, Hash};
use core::ptr::NonNull;

pub use list::{AtomicLink, ListLinkOps as HashTableLinkOps};
pub use siphasher::sip::SipHasher as DefaultHasher;

use super::list::unlink;
use super::{list, Adapter, ExclusivePointerOps, KeyAdapter, PointerOps};
use crate::init::ConstInit;
use crate::intrusive::LinkOps;

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

pub struct HashTable<A: Adapter<Link: HashTableLinkOps>, S, const N: usize> {
    buckets: [Option<NonNull<A::Link>>; N],
    adapter: A,
    state: S,
}
pub type DefaultHashTable<A, const N: usize> = HashTable<A, DefaultState, N>;
unsafe impl<A: Adapter<Link: HashTableLinkOps> + Send, S: Send, const N: usize> Send
    for HashTable<A, S, N>
where
    A::Pointer: Send,
{
}
unsafe impl<A: Adapter<Link: HashTableLinkOps> + Sync, S: Sync, const N: usize> Sync
    for HashTable<A, S, N>
where
    A::Pointer: Sync,
{
}

impl<A: Adapter<Link: HashTableLinkOps>, const N: usize> HashTable<A, DefaultState, N> {
    pub const fn new(adapter: A) -> Self {
        Self::with_hasher(adapter, ConstInit::INIT)
    }
}

impl<A: Adapter<Link: HashTableLinkOps>, S, const N: usize> HashTable<A, S, N> {
    pub const fn with_hasher(adapter: A, state: S) -> Self {
        Self {
            buckets: [None; N],
            adapter,
            state,
        }
    }

    pub fn try_insert<'a>(&mut self, ptr: A::Pointer) -> Result<(), A::Pointer>
    where
        A: KeyAdapter,
        A::Key<'a>: Hash,
        A::Value: 'a,
        S: BuildHasher,
    {
        let (value, meta) = A::Pointer::into_raw(ptr);
        unsafe {
            let bucket = self.bucket_for(&*value);
            let link = self.adapter.get_link(value);
            let link = NonNull::new_unchecked(link as *mut A::Link);
            if !link.as_ref().acquire() {
                return Err(A::Pointer::from_raw(value, meta));
            }
            link.as_ref().set_meta(meta);
            if let Some(head) = self.buckets[bucket] {
                list::insert_before(head, link);
            }
            self.buckets[bucket] = Some(link);
        }
        Ok(())
    }

    pub fn insert<'a>(&mut self, ptr: A::Pointer)
    where
        A: KeyAdapter,
        A::Key<'a>: Hash,
        A::Value: 'a,
        S: BuildHasher,
    {
        if self.try_insert(ptr).is_err() {
            panic!("Already linked");
        }
    }

    pub fn remove<'a>(&mut self, k: &impl Borrow<A::Key<'a>>) -> Option<A::Pointer>
    where
        A: KeyAdapter,
        A::Value: 'a,
        A::Key<'a>: Hash + Eq,
        S: BuildHasher,
    {
        self.raw_get(k)
            .map(move |cursor| unsafe { cursor.unlink(&mut self.buckets, &self.adapter) })
    }

    pub fn contains<'a>(&self, k: &impl Borrow<A::Key<'a>>) -> bool
    where
        A: KeyAdapter,
        A::Value: 'a,
        A::Key<'a>: Hash + Eq,
        S: BuildHasher,
    {
        self.raw_get(k).is_some()
    }

    pub fn get<'a>(&self, k: &impl Borrow<A::Key<'a>>) -> Option<&A::Value>
    where
        A: KeyAdapter,
        A::Value: 'a,
        A::Key<'a>: Hash + Eq,
        S: BuildHasher,
    {
        self.raw_get(k)
            .map(|cursor| unsafe { cursor.get_ref(&self.adapter) })
    }

    pub fn get_mut<'a>(&mut self, k: &impl Borrow<A::Key<'a>>) -> Option<&mut A::Value>
    where
        A: KeyAdapter,
        A::Value: 'a,
        A::Key<'a>: Hash + Eq,
        S: BuildHasher,
        A::Pointer: ExclusivePointerOps,
    {
        self.raw_get(k)
            .map(|cursor| unsafe { cursor.get_mut(&self.adapter) })
    }

    pub fn get_cursor<'k>(&self, k: &impl Borrow<A::Key<'k>>) -> Option<Cursor<'_, A, S, N>>
    where
        A: KeyAdapter,
        A::Value: 'k,
        A::Key<'k>: Hash + Eq,
        S: BuildHasher,
    {
        Some(Cursor {
            cursor: self.raw_get(k)?,
            table: self,
        })
    }

    pub fn get_cursor_mut<'k>(
        &mut self,
        k: &impl Borrow<A::Key<'k>>,
    ) -> Option<CursorMut<'_, A, S, N>>
    where
        A: KeyAdapter,
        A::Value: 'k,
        A::Key<'k>: Hash + Eq,
        S: BuildHasher,
    {
        Some(CursorMut {
            cursor: self.raw_get(k)?,
            table: self,
        })
    }

    pub fn front(&self) -> Option<Cursor<'_, A, S, N>> {
        Some(Cursor {
            cursor: self.raw_front()?,
            table: self,
        })
    }

    pub fn front_mut(&mut self) -> Option<CursorMut<'_, A, S, N>> {
        Some(CursorMut {
            cursor: self.raw_front()?,
            table: self,
        })
    }

    pub fn iter(&self) -> Iter<'_, A, S, N> {
        Iter {
            cursor: self.raw_front(),
            table: self,
        }
    }

    pub fn iter_mut(&mut self) -> IterMut<'_, A, S, N>
    where
        A::Pointer: ExclusivePointerOps,
    {
        IterMut {
            cursor: self.raw_front(),
            table: self,
        }
    }

    pub fn into_iter(self) -> IntoIter<A, S, N> {
        IntoIter {
            cur: self.raw_front(),
            table: self,
        }
    }

    pub fn fast_clear(&mut self) {
        self.buckets = [None; N];
    }

    pub fn clear(&mut self) {
        let mut cur = self.raw_front();
        while let Some(cursor) = cur {
            unsafe {
                let next = cursor.next(&self.buckets);
                drop(cursor.unlink(&mut self.buckets, &self.adapter));
                cur = next;
            }
        }
    }

    fn bucket_for<'a>(&self, value: &'a A::Value) -> usize
    where
        A: KeyAdapter,
        A::Key<'a>: Hash,
        S: BuildHasher,
    {
        let key = self.adapter.get_key(value);
        self.bucket_for_key(&key)
    }

    fn bucket_for_key<'a>(&self, key: &A::Key<'a>) -> usize
    where
        A: KeyAdapter,
        A::Key<'a>: Hash,
        S: BuildHasher,
    {
        let key = self.state.hash_one(&key);
        (key % N as u64) as usize
    }

    fn raw_front(&self) -> Option<RawCursor<A>> {
        for i in 0..N {
            if let Some(head) = self.buckets[i] {
                return Some(RawCursor {
                    bucket: i,
                    cur: head,
                });
            }
        }
        None
    }

    fn raw_get<'a>(&self, k: &impl Borrow<A::Key<'a>>) -> Option<RawCursor<A>>
    where
        A: KeyAdapter,
        A::Value: 'a,
        A::Key<'a>: Hash + Eq,
        S: BuildHasher,
    {
        let k = k.borrow();
        let bucket = self.bucket_for_key(k);
        let mut cur = self.buckets[bucket];
        while let Some(link_ptr) = cur {
            unsafe {
                let link = link_ptr.as_ref();
                let value = self.adapter.get_value(link);
                let key = self.adapter.get_key(&*value);
                if key == *k {
                    return Some(RawCursor {
                        bucket,
                        cur: link_ptr,
                    });
                }
                cur = link.get_next();
            }
        }
        None
    }
}
impl<A: Adapter<Link: HashTableLinkOps>, S, const N: usize> Drop for HashTable<A, S, N> {
    fn drop(&mut self) {
        self.clear()
    }
}

impl<A: Adapter<Link: HashTableLinkOps> + ConstInit, S: ConstInit, const N: usize> ConstInit
    for HashTable<A, S, N>
{
    const INIT: Self = Self::with_hasher(ConstInit::INIT, ConstInit::INIT);
}

struct RawCursor<A: Adapter<Link: HashTableLinkOps>> {
    bucket: usize,
    cur: NonNull<A::Link>,
}
impl<A: Adapter<Link: HashTableLinkOps>> Clone for RawCursor<A> {
    fn clone(&self) -> Self {
        *self
    }
}
impl<A: Adapter<Link: HashTableLinkOps>> Copy for RawCursor<A> {}

impl<A: Adapter<Link: HashTableLinkOps>> RawCursor<A> {
    unsafe fn next<const N: usize>(self, buckets: &[Option<NonNull<A::Link>>; N]) -> Option<Self> {
        if let Some(next) = self.cur.as_ref().get_next() {
            Some(Self {
                bucket: self.bucket,
                cur: next,
            })
        } else {
            for i in (self.bucket + 1)..N {
                if let Some(next) = buckets[i] {
                    return Some(Self {
                        bucket: i,
                        cur: next,
                    });
                }
            }
            None
        }
    }
    unsafe fn move_next<const N: usize>(
        &mut self,
        buckets: &[Option<NonNull<A::Link>>; N],
    ) -> bool {
        self.next(buckets).map(|next| *self = next).is_some()
    }

    unsafe fn prev<const N: usize>(self, buckets: &[Option<NonNull<A::Link>>; N]) -> Option<Self> {
        if let Some(prev) = self.cur.as_ref().get_prev() {
            Some(Self {
                bucket: self.bucket,
                cur: prev,
            })
        } else {
            for i in (0..self.bucket).rev() {
                if let Some(prev) = buckets[i] {
                    return Some(Self {
                        bucket: i,
                        cur: prev,
                    });
                }
            }
            None
        }
    }
    unsafe fn move_prev<const N: usize>(
        &mut self,
        buckets: &[Option<NonNull<A::Link>>; N],
    ) -> bool {
        self.prev(buckets).map(|prev| *self = prev).is_some()
    }

    fn get_link(self) -> NonNull<A::Link> {
        self.cur
    }

    unsafe fn get_ref<'a>(self, adapter: &A) -> &'a A::Value {
        &*adapter.get_value(self.cur.as_ptr())
    }

    unsafe fn get_mut<'a>(self, adapter: &A) -> &'a mut A::Value
    where
        A::Pointer: ExclusivePointerOps,
    {
        &mut *(adapter.get_value(self.cur.as_ptr()) as *mut A::Value)
    }

    fn bucket(self) -> usize {
        self.bucket
    }

    unsafe fn unlink<const N: usize>(
        self,
        buckets: &mut [Option<NonNull<A::Link>>; N],
        adapter: &A,
    ) -> A::Pointer {
        if buckets[self.bucket] == Some(self.cur) {
            buckets[self.bucket] = self.cur.as_ref().get_next();
        }
        let meta = unlink(self.cur);
        let value = adapter.get_value(self.cur.as_ptr());
        <A::Pointer as PointerOps>::from_raw(value, meta)
    }
}

pub struct Cursor<'a, A: Adapter<Link: HashTableLinkOps>, S, const N: usize> {
    cursor: RawCursor<A>,
    table: &'a HashTable<A, S, N>,
}
impl<'a, A: Adapter<Link: HashTableLinkOps>, S, const N: usize> Clone for Cursor<'a, A, S, N> {
    fn clone(&self) -> Self {
        *self
    }
}
impl<'a, A: Adapter<Link: HashTableLinkOps>, S, const N: usize> Copy for Cursor<'a, A, S, N> {}
impl<'a, A: Adapter<Link: HashTableLinkOps>, S, const N: usize> Cursor<'a, A, S, N> {
    pub fn next(self) -> Option<Self> {
        Some(Self {
            cursor: unsafe { self.cursor.next(&self.table.buckets)? },
            table: self.table,
        })
    }
    pub fn move_next(&mut self) -> bool {
        unsafe { self.cursor.move_next(&self.table.buckets) }
    }
    pub fn prev(self) -> Option<Self> {
        Some(Self {
            cursor: unsafe { self.cursor.prev(&self.table.buckets)? },
            table: self.table,
        })
    }
    pub fn move_prev(&mut self) -> bool {
        unsafe { self.cursor.move_prev(&self.table.buckets) }
    }

    pub fn get(&self) -> &A::Value {
        unsafe { self.cursor.get_ref(&self.table.adapter) }
    }
}

pub struct CursorMut<'a, A: Adapter<Link: HashTableLinkOps>, S, const N: usize> {
    cursor: RawCursor<A>,
    table: &'a mut HashTable<A, S, N>,
}
impl<'a, A: Adapter<Link: HashTableLinkOps>, S, const N: usize> CursorMut<'a, A, S, N> {
    pub fn next(self) -> Option<Self> {
        Some(Self {
            cursor: unsafe { self.cursor.next(&self.table.buckets)? },
            table: self.table,
        })
    }
    pub fn move_next(&mut self) -> bool {
        unsafe { self.cursor.move_next(&self.table.buckets) }
    }
    pub fn prev(self) -> Option<Self> {
        Some(Self {
            cursor: unsafe { self.cursor.prev(&self.table.buckets)? },
            table: self.table,
        })
    }
    pub fn move_prev(&mut self) -> bool {
        unsafe { self.cursor.move_prev(&self.table.buckets) }
    }

    pub fn get(&self) -> &A::Value {
        unsafe { self.cursor.get_ref(&self.table.adapter) }
    }

    pub fn get_mut(&mut self) -> &mut A::Value
    where
        A::Pointer: ExclusivePointerOps,
    {
        unsafe { self.cursor.get_mut(&mut self.table.adapter) }
    }

    pub fn unlink(self) -> A::Pointer {
        unsafe {
            self.cursor
                .unlink(&mut self.table.buckets, &self.table.adapter)
        }
    }
}

pub struct Iter<'a, A: Adapter<Link: HashTableLinkOps>, S, const N: usize> {
    cursor: Option<RawCursor<A>>,
    table: &'a HashTable<A, S, N>,
}
impl<'a, A: Adapter<Link: HashTableLinkOps>, S, const N: usize> Clone for Iter<'a, A, S, N> {
    fn clone(&self) -> Self {
        *self
    }
}
impl<'a, A: Adapter<Link: HashTableLinkOps>, S, const N: usize> Copy for Iter<'a, A, S, N> {}
impl<'a, A: Adapter<Link: HashTableLinkOps>, S, const N: usize> Iterator for Iter<'a, A, S, N> {
    type Item = &'a A::Value;
    fn next(&mut self) -> Option<Self::Item> {
        self.cursor.map(|cursor| unsafe {
            let value = cursor.get_ref(&self.table.adapter);
            self.cursor = cursor.next(&self.table.buckets);
            value
        })
    }
}
impl<'a, A: Adapter<Link: HashTableLinkOps>, S, const N: usize> IntoIterator
    for &'a HashTable<A, S, N>
{
    type Item = &'a A::Value;
    type IntoIter = Iter<'a, A, S, N>;
    fn into_iter(self) -> Self::IntoIter {
        self.iter()
    }
}
impl<'a, A: Adapter<Link: HashTableLinkOps>, S, const N: usize> IntoIterator
    for &'a mut HashTable<A, S, N>
where
    A::Pointer: ExclusivePointerOps,
{
    type Item = &'a mut A::Value;
    type IntoIter = IterMut<'a, A, S, N>;
    fn into_iter(self) -> Self::IntoIter {
        self.iter_mut()
    }
}

pub struct IterMut<
    'a,
    A: Adapter<Link: HashTableLinkOps, Pointer: ExclusivePointerOps>,
    S,
    const N: usize,
> {
    cursor: Option<RawCursor<A>>,
    table: &'a mut HashTable<A, S, N>,
}
impl<'a, A: Adapter<Link: HashTableLinkOps, Pointer: ExclusivePointerOps>, S, const N: usize>
    Iterator for IterMut<'a, A, S, N>
{
    type Item = &'a mut A::Value;
    fn next(&mut self) -> Option<Self::Item> {
        self.cursor.map(|cursor| unsafe {
            let value = cursor.get_mut(&self.table.adapter);
            self.cursor = cursor.next(&self.table.buckets);
            value
        })
    }
}

pub struct IntoIter<A: Adapter<Link: HashTableLinkOps>, S, const N: usize> {
    table: HashTable<A, S, N>,
    cur: Option<RawCursor<A>>,
}
impl<A: Adapter<Link: HashTableLinkOps>, S, const N: usize> Iterator for IntoIter<A, S, N> {
    type Item = A::Pointer;
    fn next(&mut self) -> Option<Self::Item> {
        self.cur.map(|cur| unsafe {
            let next = cur.next(&self.table.buckets);
            let ptr = cur.unlink(&mut self.table.buckets, &self.table.adapter);
            self.cur = next;
            ptr
        })
    }
}
impl<A: Adapter<Link: HashTableLinkOps>, S, const N: usize> IntoIterator for HashTable<A, S, N> {
    type Item = A::Pointer;
    type IntoIter = IntoIter<A, S, N>;
    fn into_iter(self) -> Self::IntoIter {
        self.into_iter()
    }
}

#[cfg(test)]
mod tests {
    use alloc::sync::Arc;
    use std::collections::HashSet;
    use std::prelude::v1::*;

    use super::*;

    struct A {
        link: AtomicLink<()>,
        key: u32,
        value: u32,
    }
    impl A {
        pub const fn new(value: u32) -> Self {
            Self {
                link: AtomicLink::new(),
                key: value,
                value,
            }
        }
    }
    crate::intrusive_adapter!(struct AAdapter<'a> = &'a A : A { link: AtomicLink<()> });
    crate::intrusive_adapter!(struct AAdapterMut<'a> = &'a mut A : A { link: AtomicLink<()> });
    crate::intrusive_adapter!(struct AAdapterArc = Arc<A> : A { link: AtomicLink<()> });

    impl KeyAdapter for AAdapter<'_> {
        type Key<'a> = u32;
        fn get_key<'a>(&self, value: &'a Self::Value) -> Self::Key<'a> {
            value.key
        }
    }
    impl KeyAdapter for AAdapterMut<'_> {
        type Key<'a> = u32;
        fn get_key<'a>(&self, value: &'a Self::Value) -> Self::Key<'a> {
            value.key
        }
    }
    impl KeyAdapter for AAdapterArc {
        type Key<'a> = u32;
        fn get_key<'a>(&self, value: &'a Self::Value) -> Self::Key<'a> {
            value.key
        }
    }

    fn with_hash_table<R>(f: impl FnOnce(&mut DefaultHashTable<AAdapterMut, 4>) -> R) -> R {
        let mut a0 = A::new(0);
        let mut a1 = A::new(1);
        let mut a2 = A::new(2);
        let mut table = HashTable::new(AAdapterMut::new());
        table.insert(&mut a0);
        table.insert(&mut a1);
        table.insert(&mut a2);
        f(&mut table)
    }

    #[test]
    fn insert() {
        with_hash_table(|table| {
            assert_eq!(table.get(&0u32).map(|v| v.value), Some(0));
            assert_eq!(table.get(&1u32).map(|v| v.value), Some(1));
            assert_eq!(table.get(&2u32).map(|v| v.value), Some(2));
            assert_eq!(table.get(&3u32).map(|v| v.value), None);
        })
    }

    #[test]
    fn remove() {
        with_hash_table(|table| {
            assert_eq!(table.remove(&0u32).map(|v| v.value), Some(0));
            assert_eq!(table.remove(&1u32).map(|v| v.value), Some(1));
            assert_eq!(table.remove(&2u32).map(|v| v.value), Some(2));
            assert_eq!(table.remove(&0u32).map(|v| v.value), None);
            assert_eq!(table.remove(&1u32).map(|v| v.value), None);
            assert_eq!(table.remove(&2u32).map(|v| v.value), None);
        })
    }

    #[test]
    fn cursor() {
        with_hash_table(|table| {
            let mut cur = table.front();
            let mut set = HashSet::new();
            while let Some(cursor) = cur {
                set.insert(cursor.get().value);
                cur = cursor.next();
            }
            assert!(set.contains(&0));
            assert!(set.contains(&1));
            assert!(set.contains(&2));
            assert_eq!(set.len(), 3);
        })
    }

    #[test]
    fn cursor_mut() {
        with_hash_table(|table| {
            let mut cur = table.front_mut();
            while let Some(mut cursor) = cur {
                cursor.get_mut().value += 1;
                cur = cursor.next();
            }
            let set: HashSet<_> = table.iter().map(|a| a.value).collect();
            assert!(set.contains(&1));
            assert!(set.contains(&2));
            assert!(set.contains(&3));
            assert_eq!(set.len(), 3);
        })
    }

    #[test]
    fn iter() {
        with_hash_table(|table| {
            use std::collections::HashSet;
            let set: HashSet<_> = table.iter().map(|a| a.value).collect();
            assert!(set.contains(&0));
            assert!(set.contains(&1));
            assert!(set.contains(&2));
            assert_eq!(set.len(), 3);
        })
    }

    #[test]
    fn iter_mut() {
        with_hash_table(|table| {
            for a in table.iter_mut() {
                a.value += 1;
            }
            let set: HashSet<_> = table.iter().map(|a| a.value).collect();
            assert!(set.contains(&1));
            assert!(set.contains(&2));
            assert!(set.contains(&3));
            assert_eq!(set.len(), 3);
        })
    }

    #[test]
    fn unlink() {
        with_hash_table(|table| {
            table.get_cursor_mut(&0).unwrap().unlink();
            let set: HashSet<_> = table.iter().map(|a| a.value).collect();
            assert!(!set.contains(&0));
            assert!(set.contains(&1));
            assert!(set.contains(&2));
            assert_eq!(set.len(), 2);
        })
    }

    #[test]
    fn insert_dup() {
        let a = A::new(0);
        let mut table: HashTable<_, _, 1> = HashTable::new(AAdapter::new());
        assert!(table.try_insert(&a).is_ok());
        assert!(table.try_insert(&a).is_err());
    }

    #[test]
    fn drop() {
        let mut table: HashTable<_, _, 2> = HashTable::new(AAdapterArc::new());
        let a0 = Arc::new(A::new(0));
        let a1 = Arc::new(A::new(1));
        let a2 = Arc::new(A::new(2));
        table.insert(a0.clone());
        table.insert(a1.clone());
        table.insert(a2.clone());
        assert_eq!(Arc::strong_count(&a0), 2);
        assert_eq!(Arc::strong_count(&a1), 2);
        assert_eq!(Arc::strong_count(&a2), 2);
        core::mem::drop(table);
        assert_eq!(Arc::strong_count(&a0), 1);
        assert_eq!(Arc::strong_count(&a1), 1);
        assert_eq!(Arc::strong_count(&a2), 1);
    }

    #[test]
    fn clear() {
        let mut table: HashTable<_, _, 2> = HashTable::new(AAdapterArc::new());
        let a0 = Arc::new(A::new(0));
        let a1 = Arc::new(A::new(1));
        let a2 = Arc::new(A::new(2));
        table.insert(a0.clone());
        table.insert(a1.clone());
        table.insert(a2.clone());
        assert_eq!(Arc::strong_count(&a0), 2);
        assert_eq!(Arc::strong_count(&a1), 2);
        assert_eq!(Arc::strong_count(&a2), 2);
        table.clear();
        assert_eq!(Arc::strong_count(&a0), 1);
        assert_eq!(Arc::strong_count(&a1), 1);
        assert_eq!(Arc::strong_count(&a2), 1);
        assert!(table.front().is_none());
    }

    #[test]
    fn fast_clear() {
        let mut table: HashTable<_, _, 2> = HashTable::new(AAdapterArc::new());
        let a0 = Arc::new(A::new(0));
        let a1 = Arc::new(A::new(1));
        let a2 = Arc::new(A::new(2));
        table.insert(a0.clone());
        table.insert(a1.clone());
        table.insert(a2.clone());
        assert_eq!(Arc::strong_count(&a0), 2);
        assert_eq!(Arc::strong_count(&a1), 2);
        assert_eq!(Arc::strong_count(&a2), 2);
        table.fast_clear();
        assert_eq!(Arc::strong_count(&a0), 2);
        assert_eq!(Arc::strong_count(&a1), 2);
        assert_eq!(Arc::strong_count(&a2), 2);
        assert!(table.front().is_none());
    }
}
