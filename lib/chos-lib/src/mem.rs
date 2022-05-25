use core::mem::{MaybeUninit, transmute};

use crate::pod::Pod;

pub fn maybe_uninit_init_slice<T>(slice: &[T]) -> &[MaybeUninit<T>] {
    // This is safe since MaybeUninit<T> has the same layout as T
    unsafe { transmute(slice) }
}
pub fn maybe_uninit_init_slice_mut<T: Pod>(slice: &mut [T]) -> &mut [MaybeUninit<T>] {
    // This is safe since MaybeUninit<T> has the same layout as T
    unsafe { transmute(slice) }
}
