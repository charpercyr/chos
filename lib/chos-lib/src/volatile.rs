use core::intrinsics::{unaligned_volatile_load, unaligned_volatile_store, volatile_copy_memory};
use core::marker::PhantomData;
use core::mem::{forget, transmute, ManuallyDrop, MaybeUninit};
use core::{ptr, slice};

pub use crate::access::*;
use crate::init::ConstInit;

#[repr(transparent)]
pub struct Volatile<T: ?Sized, A = ReadWrite> {
    access: PhantomData<A>,
    value: T,
}

impl<T, A> Volatile<T, A> {
    pub const fn new(value: T) -> Self {
        Self {
            access: PhantomData,
            value,
        }
    }

    pub fn write(&mut self, value: T)
    where
        A: WriteAccess,
    {
        unsafe { ptr::write_volatile(&mut self.value, value) }
    }

    pub unsafe fn write_unaligned(this: *mut Self, value: T)
    where
        A: WriteAccess,
    {
        unaligned_volatile_store(this.cast(), value)
    }

    pub fn read(&self) -> T
    where
        T: Copy,
        A: ReadAccess,
    {
        unsafe { ptr::read_volatile(&self.value) }
    }

    pub unsafe fn read_unaligned(this: *const Self) -> T
    where
        T: Copy,
        A: ReadAccess,
    {
        unaligned_volatile_load(this.cast())
    }

    pub fn update<R>(&mut self, f: impl FnOnce(&mut T) -> R) -> R
    where
        A: ReadAccess + WriteAccess,
    {
        unsafe {
            let mut v = ptr::read_volatile(&self.value);
            let r = f(&mut v);
            ptr::write_volatile(&mut self.value, v);
            r
        }
    }

    pub unsafe fn update_unaligned<R>(this: *mut Self, f: impl FnOnce(&mut T) -> R) -> R {
        let mut v = unaligned_volatile_load(this.cast());
        let r = f(&mut v);
        unaligned_volatile_store(this.cast(), v);
        r
    }

    pub fn with<R>(&self, f: impl FnOnce(&T) -> R) -> R
    where
        A: ReadAccess,
    {
        let v = unsafe { ptr::read_volatile(&self.value) };
        let r = f(&v);
        forget(v);
        r
    }

    pub fn into_inner(self) -> T {
        self.value
    }
}

impl<T: ?Sized, A> Volatile<T, A> {
    pub const fn from_ref(r: &T) -> &Self {
        unsafe { transmute(r) }
    }

    pub const fn from_mut(r: &mut T) -> &mut Self {
        unsafe { transmute(r) }
    }
}

impl<T, A, const N: usize> Volatile<[T; N], A> {
    pub const fn as_array_of_volatile(&self) -> &[Volatile<T, A>; N] {
        unsafe { transmute(self) }
    }
    pub const fn as_array_of_volatile_mut(&mut self) -> &mut [Volatile<T, A>; N] {
        unsafe { transmute(self) }
    }

    pub fn copy_array(dst: &mut Self, src: &Self) {
        unsafe { volatile_copy_memory(dst.value.as_mut_ptr(), src.value.as_ptr(), N) }
    }

    pub const fn len(&self) -> usize {
        N
    }
}

impl<T, A> Volatile<[T], A> {
    pub const fn as_slice_of_volatile(&self) -> &[Volatile<T, A>] {
        unsafe { transmute(self) }
    }

    pub const fn as_slice_of_volatile_mut(&mut self) -> &mut [Volatile<T, A>] {
        unsafe { transmute(self) }
    }

    pub fn copy_slice(dst: &mut Self, src: &Self) {
        assert!(dst.value.len() == src.value.len());
        unsafe { volatile_copy_memory(dst.value.as_mut_ptr(), src.value.as_ptr(), dst.value.len()) }
    }

    pub fn iter(&self) -> slice::Iter<'_, Volatile<T, A>> {
        self.as_slice_of_volatile().iter()
    }

    pub fn iter_mut(&mut self) -> slice::IterMut<'_, Volatile<T, A>> {
        self.as_slice_of_volatile_mut().iter_mut()
    }

    pub fn len(&self) -> usize {
        self.value.len()
    }
}

impl<'a, T, A> IntoIterator for &'a Volatile<[T], A> {
    type IntoIter = slice::Iter<'a, Volatile<T, A>>;
    type Item = &'a Volatile<T, A>;
    fn into_iter(self) -> Self::IntoIter {
        self.iter()
    }
}

impl<'a, T, A> IntoIterator for &'a mut Volatile<[T], A> {
    type IntoIter = slice::IterMut<'a, Volatile<T, A>>;
    type Item = &'a mut Volatile<T, A>;
    fn into_iter(self) -> Self::IntoIter {
        self.iter_mut()
    }
}

impl<T, P> From<T> for Volatile<T, P> {
    fn from(value: T) -> Self {
        Self {
            access: PhantomData,
            value,
        }
    }
}

impl<T: ConstInit, A> ConstInit for Volatile<T, A> {
    const INIT: Self = Self::new(T::INIT);
}

pub unsafe fn copy_volatile<T: Copy, PS: ReadAccess, PD: WriteAccess>(
    src: *const Volatile<T, PS>,
    dst: *mut Volatile<T, PD>,
    count: usize,
) {
    volatile_copy_memory::<T>(&mut (*dst).value, &(*src).value, count)
}

crate::forward_fmt!(
    impl<T: Copy, P: ReadAccess> ALL for Volatile<T, P> => T : |this: &Self| this.read()
);

#[repr(C)]
union PaddedVolatileInner<T, P, const N: usize> {
    volatile: ManuallyDrop<Volatile<T, P>>,
    _pad: MaybeUninit<[u8; N]>,
}

#[repr(transparent)]
pub struct PaddedVolatile<T, P, const N: usize> {
    inner: PaddedVolatileInner<T, P, N>,
}

impl<T, P, const N: usize> PaddedVolatile<T, P, N> {
    pub fn new(value: T) -> Self {
        Self {
            inner: PaddedVolatileInner {
                volatile: ManuallyDrop::new(Volatile::new(value)),
            },
        }
    }

    pub fn write(&mut self, value: T)
    where
        P: WriteAccess,
    {
        self.as_volatile_mut().write(value)
    }

    pub fn read(&self) -> T
    where
        T: Copy,
        P: ReadAccess,
    {
        self.as_volatile().read()
    }

    pub fn update<R>(&mut self, f: impl FnOnce(&mut T) -> R) -> R
    where
        P: ReadAccess + WriteAccess,
    {
        self.as_volatile_mut().update(f)
    }

    pub fn with<R>(&self, f: impl FnOnce(&T) -> R) -> R
    where
        P: ReadAccess,
    {
        self.as_volatile().with(f)
    }

    pub fn into_inner(self) -> T {
        let volatile = unsafe { core::ptr::read(&self.inner.volatile) };
        forget(self);
        ManuallyDrop::into_inner(volatile).into_inner()
    }

    pub fn as_volatile(&self) -> &Volatile<T, P> {
        unsafe { &self.inner.volatile }
    }

    pub fn as_volatile_mut(&mut self) -> &mut Volatile<T, P> {
        unsafe { &mut self.inner.volatile }
    }
}

unsafe impl<#[may_dangle] T, P, const N: usize> Drop for PaddedVolatile<T, P, N> {
    fn drop(&mut self) {
        unsafe { ManuallyDrop::drop(&mut self.inner.volatile) }
    }
}

crate::forward_fmt!(
    impl<T: Copy, P: ReadAccess, const N: usize> ALL for PaddedVolatile<T, P, N> => T : |this: &Self| this.read()
);
