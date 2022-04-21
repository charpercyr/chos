use core::alloc::{AllocError, Layout};
use core::fmt;
use core::future::Future;
use core::hash::Hash;
use core::marker::{PhantomData, Unsize};
use core::ops::{CoerceUnsized, Deref, DerefMut};
use core::pin::Pin;
use core::ptr::NonNull;
use core::task::{Context, Poll};

use intrusive_collections::{ExclusivePointerOps, PointerOps};

use super::{handle_alloc_error, ConstPool, Pool};
use crate::intrusive::DefaultPointerOps;

pub struct PoolBox<T: ?Sized, P: Pool<T>> {
    ptr: NonNull<T>,
    alloc: P,
    value: PhantomData<T>,
}

impl<T: ?Sized, P: Pool<T>> PoolBox<T, P> {
    pub fn try_new_in(value: T, alloc: P) -> Result<Self, AllocError>
    where
        T: Sized,
    {
        let ptr = unsafe { alloc.allocate()? };
        unsafe { core::ptr::write(ptr.as_ptr(), value) };
        Ok(Self {
            ptr,
            alloc,
            value: PhantomData,
        })
    }

    pub fn new_in(value: T, alloc: P) -> Self
    where
        T: Sized,
    {
        Self::try_new_in(value, alloc).unwrap_or_else(|_| handle_alloc_error(Layout::new::<T>()))
    }

    pub fn try_pin_in(value: T, alloc: P) -> Result<Pin<Self>, AllocError>
    where
        T: Sized,
    {
        Self::try_new_in(value, alloc).map(|b| unsafe { Pin::new_unchecked(b) })
    }

    pub fn pin_in(value: T, alloc: P) -> Pin<Self>
    where
        T: Sized,
    {
        unsafe { Pin::new_unchecked(Self::new_in(value, alloc)) }
    }

    pub fn into_raw_with_allocator(this: Self) -> (*mut T, P) {
        let ptr = this.ptr.as_ptr();
        let alloc = unsafe { core::ptr::read(&this.alloc) };
        core::mem::forget(this);
        (ptr, alloc)
    }

    pub fn leak_with_allocator<'a>(this: Self) -> (&'a mut T, P) {
        let (ptr, alloc) = Self::into_raw_with_allocator(this);
        (unsafe { &mut *ptr }, alloc)
    }

    pub unsafe fn from_raw_in(ptr: *mut T, alloc: P) -> Self {
        Self {
            ptr: NonNull::new_unchecked(ptr),
            alloc,
            value: PhantomData,
        }
    }

    pub fn get_ref(&self) -> &T {
        unsafe { self.ptr.as_ref() }
    }

    pub fn get_mut(&mut self) -> &mut T {
        unsafe { self.ptr.as_mut() }
    }

    pub fn as_ptr(&self) -> *mut T {
        self.ptr.as_ptr()
    }

    pub fn into_inner(self) -> T
    where
        T: Sized,
    {
        unsafe {
            let layout = Layout::for_value(self.get_ref());
            let value = core::ptr::read(self.ptr.as_ptr());
            self.alloc.deallocate(self.ptr, layout);
            value
        }
    }

    #[cfg(feature = "alloc")]
    pub fn into_box(this: Self) -> alloc::boxed::Box<T, P>
    where
        P: alloc::alloc::Allocator,
    {
        unsafe {
            let ptr = this.ptr;
            let alloc = core::ptr::read(&this.alloc);
            core::mem::forget(this);
            alloc::boxed::Box::from_raw_in(ptr.as_ptr(), alloc)
        }
    }
}

impl<T: ?Sized, P: ConstPool<T>> PoolBox<T, P> {
    pub fn try_new(value: T) -> Result<Self, AllocError> where T: Sized {
        Self::try_new_in(value, P::INIT)
    }

    pub fn try_pin(value: T) -> Result<Pin<Self>, AllocError> where T: Sized {
        Self::try_new(value).map(|b| unsafe { Pin::new_unchecked(b) })
    }

    pub fn new(value: T) -> Self where T: Sized {
        Self::new_in(value, P::INIT)
    }

    pub fn pin(value: T) -> Pin<Self> where T: Sized {
        unsafe { Pin::new_unchecked(Self::new(value)) }
    }

    pub fn into_raw(this: Self) -> *mut T {
        let ptr = this.ptr.as_ptr();
        core::mem::forget(this);
        ptr
    }

    pub unsafe fn from_raw(raw: *mut T) -> Self {
        Self {
            ptr: NonNull::new_unchecked(raw),
            alloc: P::INIT,
            value: PhantomData,
        }
    }

    pub fn leak<'a>(this: Self) -> &'a mut T {
        unsafe { &mut *Self::into_raw(this) }
    }
}

impl<T: ?Sized, P: Pool<T>> Drop for PoolBox<T, P> {
    fn drop(&mut self) {
        let layout = Layout::for_value(self.get_ref());
        unsafe {
            core::ptr::drop_in_place(self.ptr.as_ptr());
            self.alloc.deallocate(self.ptr, layout);
        }
    }
}

impl<T: ?Sized, P: Pool<T>> Deref for PoolBox<T, P> {
    type Target = T;
    fn deref(&self) -> &Self::Target {
        self.get_ref()
    }
}

impl<T: ?Sized, P: Pool<T>> DerefMut for PoolBox<T, P> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.get_mut()
    }
}

impl<T: Clone + ?Sized, P: Pool<T> + Clone> Clone for PoolBox<T, P> {
    fn clone(&self) -> Self {
        Self::new_in(self.get_ref().clone(), self.alloc.clone())
    }
}

impl<T: PartialEq + ?Sized, P: Pool<T>> PartialEq for PoolBox<T, P> {
    fn eq(&self, other: &Self) -> bool {
        <T as PartialEq>::eq(self, other)
    }
}
impl<T: PartialEq + ?Sized, P: Pool<T>> PartialEq<T> for PoolBox<T, P> {
    fn eq(&self, other: &T) -> bool {
        <T as PartialEq>::eq(self, other)
    }
}
impl<T: Eq + ?Sized, P: Pool<T>> Eq for PoolBox<T, P> {}

impl<T: PartialOrd + ?Sized, P: Pool<T>> PartialOrd for PoolBox<T, P> {
    fn partial_cmp(&self, other: &Self) -> Option<core::cmp::Ordering> {
        <T as PartialOrd>::partial_cmp(self, other)
    }
}
impl<T: PartialOrd + ?Sized, P: Pool<T>> PartialOrd<T> for PoolBox<T, P> {
    fn partial_cmp(&self, other: &T) -> Option<core::cmp::Ordering> {
        <T as PartialOrd>::partial_cmp(self, other)
    }
}
impl<T: Ord + ?Sized, P: Pool<T>> Ord for PoolBox<T, P> {
    fn cmp(&self, other: &Self) -> core::cmp::Ordering {
        <T as Ord>::cmp(self, other)
    }
}

impl<T: Hash + ?Sized, P: Pool<T>> Hash for PoolBox<T, P> {
    fn hash<H: core::hash::Hasher>(&self, state: &mut H) {
        T::hash(&*self, state)
    }
}

impl<T: ?Sized, P: Pool<T> + Unpin> Unpin for PoolBox<T, P> {}
unsafe impl<T: Send, P: Pool<T> + Send> Send for PoolBox<T, P> {}
unsafe impl<T: Sync, P: Pool<T> + Sync> Sync for PoolBox<T, P> {}

impl<T, U, P> CoerceUnsized<PoolBox<U, P>> for PoolBox<T, P>
where
    T: Unsize<U> + ?Sized,
    U: ?Sized,
    P: Pool<T> + Pool<U>,
{
}

macro_rules! fmt {
    ($($fmt:ident),* $(,)?) => {
        $(
            impl<T: fmt::$fmt + ?Sized, P: Pool<T>> fmt::$fmt for PoolBox<T, P> {
                fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
                    fmt::$fmt::fmt(self.get_ref(), f)
                }
            }
        )*
    };
}
fmt!(Debug, Display, Binary, Octal, LowerHex, UpperHex, LowerExp, UpperExp,);

impl<T: ?Sized, P: Pool<T>> fmt::Pointer for PoolBox<T, P> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Pointer::fmt(&self.ptr, f)
    }
}

unsafe impl<T: ?Sized, P: ConstPool<T>> PointerOps for DefaultPointerOps<PoolBox<T, P>> {
    type Value = T;
    type Pointer = PoolBox<T, P>;

    unsafe fn from_raw(&self, value: *const Self::Value) -> Self::Pointer {
        PoolBox::from_raw(value as *mut Self::Value)
    }

    fn into_raw(&self, ptr: Self::Pointer) -> *const Self::Value {
        PoolBox::into_raw(ptr)
    }
}
unsafe impl<T: ?Sized, P: ConstPool<T>> ExclusivePointerOps for DefaultPointerOps<PoolBox<T, P>> {}

impl<T: Future + Unpin + ?Sized, P: Pool<T> + Unpin> Future for PoolBox<T, P> {
    type Output = T::Output;
    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        T::poll(Pin::new(&mut *self), cx)
    }
}

impl<T: Iterator, P: Pool<T>> Iterator for PoolBox<T, P> {
    type Item = T::Item;
    fn next(&mut self) -> Option<Self::Item> {
        T::next(&mut *self)
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        T::size_hint(&*self)
    }

    fn nth(&mut self, n: usize) -> Option<Self::Item> {
        T::nth(&mut *self, n)
    }

    fn last(self) -> Option<Self::Item> {
        T::last(self.into_inner())
    }
}
impl<T: DoubleEndedIterator, P: Pool<T>> DoubleEndedIterator for PoolBox<T, P> {
    fn next_back(&mut self) -> Option<Self::Item> {
        T::next_back(&mut *self)
    }
    fn nth_back(&mut self, n: usize) -> Option<Self::Item> {
        T::nth_back(&mut *self, n)
    }
}
