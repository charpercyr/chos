use alloc::vec::Vec;
use core::marker::PhantomData;
use core::mem::forget;
use core::ops::{Deref, DerefMut, Index, IndexMut};
use core::slice::{from_raw_parts, from_raw_parts_mut, SliceIndex};

use chos_lib::{ReadAccess, ReadWrite, WriteAccess};

pub struct BufOwn<A = ReadWrite> {
    data: *mut u8,
    len: usize,
    cap: usize,
    drop: Option<unsafe fn(&mut Self)>,
    access: PhantomData<A>,
}

impl<A> BufOwn<A> {
    pub unsafe fn from_raw_parts(data: *mut u8, len: usize) -> Self {
        Self {
            data,
            len,
            cap: len,
            drop: None,
            access: PhantomData,
        }
    }

    pub unsafe fn from_raw_parts_drop(data: *mut u8, len: usize, drop: fn(&mut Self)) -> Self {
        Self {
            data,
            len,
            cap: len,
            drop: Some(drop),
            access: PhantomData,
        }
    }

    pub fn from_vec(v: Vec<u8>) -> Self {
        let (data, len, cap) = Vec::into_raw_parts(v);
        Self {
            data,
            len,
            cap,
            drop: Some(Self::drop_from_vec),
            access: PhantomData,
        }
    }

    pub fn as_slice(&self) -> &[u8]
    where
        A: ReadAccess,
    {
        unsafe { from_raw_parts(self.data, self.len) }
    }

    pub fn as_slice_mut(&mut self) -> &mut [u8]
    where
        A: ReadAccess + WriteAccess,
    {
        unsafe { from_raw_parts_mut(self.data, self.len) }
    }

    pub fn into_raw_parts(self) -> (*mut u8, usize, usize, Option<unsafe fn(&mut Self)>) {
        let res = (self.data, self.len, self.cap, self.drop);
        core::mem::forget(self);
        res
    }

    pub unsafe fn into_vec(self) -> Vec<u8> {
        let Self { data, len, cap, .. } = self;
        let v = Vec::from_raw_parts(data, len, cap);
        forget(self);
        v
    }

    unsafe fn drop_from_vec(&mut self) {
        drop(Vec::from_raw_parts(self.data, self.len, self.cap))
    }
}

impl<A: ReadAccess> Deref for BufOwn<A> {
    type Target = [u8];
    fn deref(&self) -> &Self::Target {
        self.as_slice()
    }
}

impl<A: ReadAccess + WriteAccess> DerefMut for BufOwn<A> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.as_slice_mut()
    }
}

impl<A: ReadAccess, I: SliceIndex<[u8]>> Index<I> for BufOwn<A> {
    type Output = I::Output;
    fn index(&self, index: I) -> &Self::Output {
        self.as_slice().index(index)
    }
}

impl<A: ReadAccess + WriteAccess, I: SliceIndex<[u8]>> IndexMut<I> for BufOwn<A> {
    fn index_mut(&mut self, index: I) -> &mut Self::Output {
        self.as_slice_mut().index_mut(index)
    }
}

impl<A> Drop for BufOwn<A> {
    fn drop(&mut self) {
        if let Some(drop) = self.drop {
            unsafe { drop(self) }
        }
    }
}
