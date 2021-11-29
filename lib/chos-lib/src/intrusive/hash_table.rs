use core::borrow::Borrow;
use core::hash::{BuildHasher, Hash};
use core::ptr::NonNull;

pub use list::{AtomicLink, ListLinkOps as HashTableLinkOps};
pub use siphasher::sip::SipHasher as DefaultHasher;

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
        self.raw_get(k).map(move |(bucket, link)| unsafe {
            if self.buckets[bucket] == Some(link) {
                self.buckets[bucket] = link.as_ref().get_next();
            }
            todo!()
        })
    }

    pub fn get<'a>(&self, k: &impl Borrow<A::Key<'a>>) -> Option<&A::Value>
    where
        A: KeyAdapter,
        A::Value: 'a,
        A::Key<'a>: Hash + Eq,
        S: BuildHasher,
    {
        self.raw_get(k)
            .map(|(_, link)| unsafe { &*self.adapter.get_value(link.as_ref()) })
    }

    pub fn get_mut<'a>(&mut self, k: &impl Borrow<A::Key<'a>>) -> Option<&mut A::Value>
    where
        A: KeyAdapter,
        A::Value: 'a,
        A::Key<'a>: Hash + Eq,
        S: BuildHasher,
        A::Pointer: ExclusivePointerOps,
    {
        self.raw_get(k).map(|(_, link)| unsafe {
            &mut *(self.adapter.get_value(link.as_ref()) as *mut A::Value)
        })
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

    pub fn raw_get<'a>(&self, k: &impl Borrow<A::Key<'a>>) -> Option<(usize, NonNull<A::Link>)>
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
                    return Some((bucket, link_ptr));
                }
                cur = link.get_next();
            }
        }
        None
    }
}
impl<A: Adapter<Link: HashTableLinkOps> + ConstInit, S: ConstInit, const N: usize> ConstInit
    for HashTable<A, S, N>
{
    const INIT: Self = Self::with_hasher(ConstInit::INIT, ConstInit::INIT);
}

#[cfg(test)]
mod tests {
    use std::prelude::v1::*;

    use super::*;

    struct A {
        link: AtomicLink<()>,
        value: u32,
    }
    impl A {
        pub const fn new(value: u32) -> Self {
            Self {
                link: AtomicLink::new(),
                value,
            }
        }
    }
    crate::intrusive_adapter!(struct AAdapter<'a> = &'a A : A { link: AtomicLink<()> });
    impl KeyAdapter for AAdapter<'_> {
        type Key<'a> = u32;
        fn get_key<'a>(&self, value: &'a Self::Value) -> Self::Key<'a> {
            value.value
        }
    }

    #[test]
    fn test_insert() {
        let a0 = A::new(0);
        let a1 = A::new(1);
        let mut table = HashTable::<_, _, 8>::new(AAdapter::new());
        table.insert(&a0);
        table.insert(&a1);
        assert_eq!(table.get(&0u32).map(|v| v.value), Some(0));
        assert_eq!(table.get(&1u32).map(|v| v.value), Some(1));
        assert_eq!(table.get(&2u32).map(|v| v.value), None);
    }
}
