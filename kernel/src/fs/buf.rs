use alloc::boxed::Box;
use alloc::vec::Vec;
use core::marker::PhantomData;
use core::mem::forget;
use core::ops::{Deref, DerefMut, Index, IndexMut};
use core::slice::{from_raw_parts, from_raw_parts_mut, SliceIndex};

use chos_lib::{ReadAccess, ReadWrite, WriteAccess};

pub struct BufOwn<T, A = ReadWrite> {
    data: *mut T,
    len: usize,
    cap: usize,
    drop: Option<unsafe fn(&mut Self)>,
    access: PhantomData<A>,
}

impl<T, A> BufOwn<T, A> {
    pub unsafe fn from_raw_parts(data: *mut T, len: usize) -> Self {
        Self {
            data,
            len,
            cap: len,
            drop: None,
            access: PhantomData,
        }
    }

    pub unsafe fn from_raw_parts_drop(
        data: *mut T,
        len: usize,
        drop: unsafe fn(&mut Self),
    ) -> Self {
        Self {
            data,
            len,
            cap: len,
            drop: Some(drop),
            access: PhantomData,
        }
    }

    pub unsafe fn from_mut_slice(slice: &mut [T]) -> Self {
        Self::from_raw_parts(slice.as_mut_ptr(), slice.len())
    }

    pub unsafe fn from_mut_slice_drop(slice: &mut [T], drop: unsafe fn(&mut Self)) -> Self {
        Self::from_raw_parts_drop(slice.as_mut_ptr(), slice.len(), drop)
    }

    pub fn from_vec(v: Vec<T>) -> Self {
        let (data, len, cap) = Vec::into_raw_parts(v);
        Self {
            data,
            len,
            cap,
            drop: Some(Self::drop_from_vec),
            access: PhantomData,
        }
    }

    pub fn from_box(b: Box<[T]>) -> Self {
        unsafe { Self::from_mut_slice_drop(Box::leak(b), Self::drop_from_box) }
    }

    pub fn as_slice(&self) -> &[T]
    where
        A: ReadAccess,
    {
        unsafe { from_raw_parts(self.data, self.len) }
    }

    pub fn as_slice_mut(&mut self) -> &mut [T]
    where
        A: ReadAccess + WriteAccess,
    {
        unsafe { from_raw_parts_mut(self.data, self.len) }
    }

    pub fn into_raw_parts(self) -> (*mut T, usize, usize, Option<unsafe fn(&mut Self)>) {
        let res = (self.data, self.len, self.cap, self.drop);
        core::mem::forget(self);
        res
    }

    pub unsafe fn into_vec(self) -> Vec<T> {
        let Self { data, len, cap, .. } = self;
        let v = Vec::from_raw_parts(data, len, cap);
        forget(self);
        v
    }

    unsafe fn drop_from_box(&mut self) {
        drop(Box::from_raw(from_raw_parts_mut(self.data, self.len)))
    }

    unsafe fn drop_from_vec(&mut self) {
        drop(Vec::from_raw_parts(self.data, self.len, self.cap))
    }
}

unsafe impl<T: Send, A> Send for BufOwn<T, A> {}
unsafe impl<T: Sync, A> Sync for BufOwn<T, A> {}

impl<T, A: ReadAccess> Deref for BufOwn<T, A> {
    type Target = [T];
    fn deref(&self) -> &Self::Target {
        self.as_slice()
    }
}

impl<T, A: ReadAccess + WriteAccess> DerefMut for BufOwn<T, A> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.as_slice_mut()
    }
}

impl<T, A: ReadAccess, I: SliceIndex<[T]>> Index<I> for BufOwn<T, A> {
    type Output = I::Output;
    fn index(&self, index: I) -> &Self::Output {
        self.as_slice().index(index)
    }
}

impl<T, A: ReadAccess + WriteAccess, I: SliceIndex<[T]>> IndexMut<I> for BufOwn<T, A> {
    fn index_mut(&mut self, index: I) -> &mut Self::Output {
        self.as_slice_mut().index_mut(index)
    }
}

impl<T, A> Drop for BufOwn<T, A> {
    fn drop(&mut self) {
        if let Some(drop) = self.drop {
            unsafe { drop(self) }
        }
    }
}
