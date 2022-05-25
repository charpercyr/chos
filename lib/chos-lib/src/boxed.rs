use alloc::boxed::Box;
use core::alloc::AllocError;
use core::intrinsics::transmute;
use core::mem::MaybeUninit;

use crate::pod::Pod;

pub fn try_new_boxed_uninit_array<T, const N: usize>(
) -> Result<Box<[MaybeUninit<T>; N]>, AllocError> {
    let b = Box::<[T], _>::try_new_uninit_slice(N)?;
    let ptr = Box::into_raw(b);
    Ok(unsafe { Box::from_raw(ptr.cast()) })
}

pub fn try_new_boxed_array<T: Pod, const N: usize>() -> Result<Box<[T; N]>, AllocError> {
    unsafe { transmute(try_new_boxed_uninit_array::<T, N>()) }
}
