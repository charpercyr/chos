use core::intrinsics::volatile_copy_memory;
use core::marker::PhantomData;
use core::ptr;

pub trait WriteAccess {}
pub trait ReadAccess {}

pub struct NoAccess;
pub struct WriteOnly;
pub struct ReadOnly;
pub struct ReadWrite;

impl WriteAccess for WriteOnly {}
impl WriteAccess for ReadWrite {}
impl ReadAccess for ReadOnly {}
impl ReadAccess for ReadWrite {}

#[repr(transparent)]
pub struct Volatile<T, P = ReadWrite>(T, PhantomData<P>);

impl<T, P> Volatile<T, P> {
    pub const fn new(value: T) -> Self {
        Self(value, PhantomData)
    }

    pub fn write(&mut self, value: T)
    where
        P: WriteAccess,
    {
        unsafe { ptr::write_volatile(&mut self.0, value) }
    }

    pub fn read(&self) -> T
    where
        T: Copy,
        P: ReadAccess,
    {
        unsafe { ptr::read_volatile(&self.0) }
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
    use core::mem::transmute;
    volatile_copy_memory::<T>(transmute(dst), transmute(src), count)
}

crate::forward_fmt!(
    impl<T: Copy, P: ReadAccess> ALL for Volatile<T, P> => T : |this: &Self| this.read()
);
