use core::alloc::{AllocError, Layout};
use core::convert::TryFrom;
use core::fmt;
use core::marker::{PhantomData, Unpin};
use core::ops::Deref;
use core::ptr::NonNull;
use core::sync::atomic::{AtomicUsize, Ordering};

use super::{handle_alloc_error, ConstPool, Pool, PoolBox};
use crate::init::ConstInit;

pub struct IArcCount {
    count: AtomicUsize,
}

impl ConstInit for IArcCount {
    #[allow(clippy::declare_interior_mutable_const)]
    const INIT: Self = Self {
        count: AtomicUsize::new(0),
    };
}

impl fmt::Debug for IArcCount {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("IArcCount").finish()
    }
}

pub trait IArcAdapter {
    fn count(&self) -> &IArcCount;
}

pub struct IArc<T: IArcAdapter, P: Pool<T>> {
    ptr: NonNull<T>,
    alloc: P,
    value: PhantomData<T>,
}

impl<T: IArcAdapter, P: Pool<T>> IArc<T, P> {
    pub fn try_new_in(value: T, alloc: P) -> Result<Self, AllocError> {
        value.count().count.fetch_add(1, Ordering::Relaxed);
        let ptr = unsafe { alloc.allocate()? };
        unsafe { core::ptr::write(ptr.as_ptr(), value) };
        Ok(Self {
            ptr,
            alloc,
            value: PhantomData,
        })
    }

    pub fn new_in(value: T, alloc: P) -> Self {
        let r = Self::try_new_in(value, alloc);
        r.unwrap_or_else(|_| handle_alloc_error(Layout::new::<T>()))
    }

    pub fn into_raw_with_allocator(this: Self) -> (*const T, P) {
        let ptr = this.ptr.as_ptr();
        let alloc = unsafe { core::ptr::read(&this.alloc) };
        core::mem::forget(this);
        (ptr, alloc)
    }

    pub unsafe fn from_raw_in(ptr: *const T, alloc: P) -> Self {
        Self {
            ptr: NonNull::new_unchecked(ptr as _),
            alloc,
            value: PhantomData,
        }
    }

    pub fn get_ref(&self) -> &T {
        unsafe { self.ptr.as_ref() }
    }

    pub fn get_mut(this: &mut Self) -> Option<&mut T> {
        this.is_unique()
            .then(move || unsafe { Self::get_mut_unchecked(this) })
    }

    pub unsafe fn get_mut_unchecked(this: &mut Self) -> &mut T {
        this.ptr.as_mut()
    }

    pub fn count(&self) -> usize {
        self.get_count().load(Ordering::Relaxed)
    }

    pub fn is_unique(&self) -> bool {
        self.count() == 1
    }

    fn get_count(&self) -> &AtomicUsize {
        unsafe { &self.ptr.as_ref().count().count }
    }

    pub fn from_pool_box(b: PoolBox<T, P>) -> Self {
        let (ptr, alloc) = PoolBox::leak_with_allocator(b);
        let count = ptr.count();
        count.count.fetch_add(1, Ordering::Relaxed);
        Self {
            ptr: ptr.into(),
            alloc,
            value: PhantomData,
        }
    }

    pub fn into_pool_box(this: Self) -> Result<PoolBox<T, P>, Self> {
        if this.is_unique() {
            this.get_count().fetch_sub(1, Ordering::Relaxed);
            let (ptr, alloc) = Self::into_raw_with_allocator(this);
            unsafe { Ok(PoolBox::from_raw_in(ptr as _, alloc)) }
        } else {
            Err(this)
        }
    }
}

impl<T: IArcAdapter, P: ConstPool<T>> IArc<T, P> {
    pub fn new(value: T) -> Self {
        Self::try_new_in(value, P::INIT)
            .unwrap_or_else(|_| super::handle_alloc_error(Layout::new::<T>()))
    }

    pub fn try_new(value: T) -> Result<Self, AllocError> {
        Self::try_new_in(value, P::INIT)
    }

    pub fn into_raw(this: Self) -> *const T {
        let ptr = this.ptr.as_ptr();
        core::mem::forget(this);
        ptr
    }

    pub unsafe fn from_raw(raw: *const T) -> Self {
        Self {
            ptr: NonNull::new_unchecked(raw as *mut T),
            alloc: P::INIT,
            value: PhantomData,
        }
    }
}

impl<T: IArcAdapter, P: Pool<T>> Drop for IArc<T, P> {
    fn drop(&mut self) {
        if self.get_count().fetch_sub(1, Ordering::Release) != 1 {
            return;
        }

        self.get_count().load(Ordering::Acquire);
        unsafe {
            core::ptr::drop_in_place(self.ptr.as_ptr());
            self.alloc.deallocate(self.ptr)
        }
    }
}

impl<T: IArcAdapter, P: Pool<T> + Clone> Clone for IArc<T, P> {
    fn clone(&self) -> Self {
        self.get_count().fetch_add(1, Ordering::Relaxed);
        Self {
            ptr: self.ptr,
            alloc: P::clone(&self.alloc),
            value: PhantomData,
        }
    }
}

impl<T: IArcAdapter, P: Pool<T>> Deref for IArc<T, P> {
    type Target = T;
    fn deref(&self) -> &Self::Target {
        self.get_ref()
    }
}

impl<T: IArcAdapter + PartialEq, P: Pool<T>> PartialEq for IArc<T, P> {
    fn eq(&self, other: &Self) -> bool {
        <T as PartialEq>::eq(self, other)
    }
}
impl<T: IArcAdapter + PartialEq, P: Pool<T>> PartialEq<T> for IArc<T, P> {
    fn eq(&self, other: &T) -> bool {
        <T as PartialEq>::eq(self, other)
    }
}
impl<T: IArcAdapter + Eq, P: Pool<T>> Eq for IArc<T, P> {}

impl<T: IArcAdapter + PartialOrd, P: Pool<T>> PartialOrd for IArc<T, P> {
    fn partial_cmp(&self, other: &Self) -> Option<core::cmp::Ordering> {
        <T as PartialOrd>::partial_cmp(self, other)
    }
}
impl<T: IArcAdapter + PartialOrd, P: Pool<T>> PartialOrd<T> for IArc<T, P> {
    fn partial_cmp(&self, other: &T) -> Option<core::cmp::Ordering> {
        <T as PartialOrd>::partial_cmp(self, other)
    }
}
impl<T: IArcAdapter + Ord, P: Pool<T>> Ord for IArc<T, P> {
    fn cmp(&self, other: &Self) -> core::cmp::Ordering {
        <T as Ord>::cmp(self, other)
    }
}

impl<T: IArcAdapter, P: Unpin + Pool<T>> Unpin for IArc<T, P> {}
unsafe impl<T: IArcAdapter + Sync + Send, P: Pool<T> + Sync> Sync for IArc<T, P> {}
unsafe impl<T: IArcAdapter + Sync + Send, P: Pool<T> + Send> Send for IArc<T, P> {}

macro_rules! fmt {
    ($($fmt:ident),* $(,)?) => {
        $(
            impl<T: IArcAdapter + fmt::$fmt, P: Pool<T>> fmt::$fmt for IArc<T, P> {
                fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
                    fmt::$fmt::fmt(self.get_ref(), f)
                }
            }
        )*
    };
}

fmt!(Debug, Display, Binary, Octal, LowerHex, UpperHex, LowerExp, UpperExp,);

impl<T: IArcAdapter, P: Pool<T>> fmt::Pointer for IArc<T, P> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        fmt::Pointer::fmt(&self.ptr, f)
    }
}

impl<T: IArcAdapter, P: Pool<T>> crate::intrusive::PointerOps for IArc<T, P> {
    type Metadata = P;
    type Target = T;
    fn into_raw(this: Self) -> (*const Self::Target, Self::Metadata) {
        Self::into_raw_with_allocator(this)
    }
    unsafe fn from_raw(ptr: *const Self::Target, meta: Self::Metadata) -> Self {
        Self::from_raw_in(ptr, meta)
    }
}

impl<T: IArcAdapter, P: Pool<T>> From<PoolBox<T, P>> for IArc<T, P> {
    fn from(b: PoolBox<T, P>) -> Self {
        Self::from_pool_box(b)
    }
}

impl<T: IArcAdapter, P: Pool<T>> TryFrom<IArc<T, P>> for PoolBox<T, P> {
    type Error = IArc<T, P>;
    fn try_from(value: IArc<T, P>) -> Result<Self, Self::Error> {
        IArc::into_pool_box(value)
    }
}
