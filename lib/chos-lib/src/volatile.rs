use core::intrinsics::{unaligned_volatile_load, unaligned_volatile_store, volatile_copy_memory};
use core::marker::PhantomData;
use core::mem::{forget, transmute, ManuallyDrop};
use core::ptr;

pub use crate::access::*;

#[repr(transparent)]
pub struct Volatile<T, P = ReadWrite>(T, PhantomData<P>);

impl<T, P> Volatile<T, P> {
    pub const fn new(value: T) -> Self {
        Self(value, PhantomData)
    }

    pub const fn from_ref(r: &T) -> &Self {
        unsafe { transmute(r) }
    }

    pub const fn from_mut(r: &mut T) -> &mut Self {
        unsafe { transmute(r) }
    }

    pub fn write(&mut self, value: T)
    where
        P: WriteAccess,
    {
        unsafe { ptr::write_volatile(&mut self.0, value) }
    }

    pub unsafe fn write_unaligned(this: *mut Self, value: T)
    where
        P: WriteAccess,
    {
        unaligned_volatile_store(this.cast(), value)
    }

    pub fn read(&self) -> T
    where
        T: Copy,
        P: ReadAccess,
    {
        unsafe { ptr::read_volatile(&self.0) }
    }

    pub unsafe fn read_unaligned(this: *const Self) -> T
    where
        T: Copy,
        P: ReadAccess,
    {
        unaligned_volatile_load(this.cast())
    }

    pub fn update(&mut self, f: impl FnOnce(&mut T))
    where
        P: ReadAccess + WriteAccess,
    {
        unsafe {
            let mut v = ptr::read_volatile(&self.0);
            f(&mut v);
            ptr::write_volatile(&mut self.0, v);
        }
    }

    pub fn into_inner(self) -> T {
        self.0
    }
}

impl<T, P> From<T> for Volatile<T, P> {
    fn from(value: T) -> Self {
        Self(value, PhantomData)
    }
}

pub unsafe fn copy_volatile<T: Copy, PS: ReadAccess, PD: WriteAccess>(
    src: *const Volatile<T, PS>,
    dst: *mut Volatile<T, PD>,
    count: usize,
) {
    volatile_copy_memory::<T>(&mut (*dst).0, &(*src).0, count)
}

crate::forward_fmt!(
    impl<T: Copy, P: ReadAccess> ALL for Volatile<T, P> => T : |this: &Self| this.read()
);

#[repr(C)]
union PaddedVolatileInner<T, P, const N: usize> {
    volatile: ManuallyDrop<Volatile<T, P>>,
    _pad: [u8; N],
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

    pub fn update(&mut self, f: impl FnOnce(&mut T))
    where
        P: ReadAccess + WriteAccess,
    {
        self.as_volatile_mut().update(f)
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
