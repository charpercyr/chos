
use core::alloc::AllocError;
use core::fmt;
use core::marker::PhantomData;
use core::ops::Deref;
use core::ptr::NonNull;
use core::sync::atomic::{AtomicUsize, Ordering};

use super::{Pool, GlobalPool};

pub struct PoolArcCount {
    count: AtomicUsize,
}

impl PoolArcCount {
    pub const INIT: Self = Self {
        count: AtomicUsize::new(0),
    };
}

impl fmt::Debug for PoolArcCount {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("PoolArcCount").finish()
    }
}

pub trait PoolArcAdapter {
    fn count(&self) -> &PoolArcCount;
}

pub struct PoolArc<T: PoolArcAdapter, P: Pool<T>> {
    ptr: NonNull<T>,
    alloc: P,
    value: PhantomData<T>,
}

impl<T: PoolArcAdapter, P: Pool<T>> PoolArc<T, P> {
    pub fn try_new_in(value: T, alloc: P) -> Result<Self, AllocError> {
        value.count().count.fetch_add(1, Ordering::Relaxed);
        let ptr = unsafe { alloc.allocate()?.cast() };
        unsafe { core::ptr::write(ptr.as_ptr(), value) };
        Ok(Self {
            ptr,
            alloc,
            value: PhantomData,
        })
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

    pub fn as_ref(&self) -> &T {
        unsafe { self.ptr.as_ref() }
    }

    pub fn get_mut(this: &mut Self) -> Option<&mut T> {
        this.is_unique().then(move || unsafe { Self::get_mut_unchecked(this) })
    }

    pub unsafe fn get_mut_unchecked(this: &mut Self) -> &mut T {
        this.ptr.as_mut()
    }

    pub fn count(&self) -> usize {
        self.get_count().load(Ordering::Relaxed)
    }

    pub fn is_unique(&self) -> bool {
        self.get_count().load(Ordering::Relaxed) == 1
    }

    fn get_count(&self) -> &AtomicUsize {
        unsafe { &self.ptr.as_ref().count().count }
    }
}

impl<T: PoolArcAdapter, P: GlobalPool<T>> PoolArc<T, P> {
    pub fn new(value: T) -> Self {
        Self::try_new_in(value, P::VALUE).unwrap_or_else(|_| P::handle_alloc_error())
    }

    pub fn try_new(value: T) -> Result<Self, AllocError> {
        Self::try_new_in(value, P::VALUE)
    }

    pub fn into_raw(this: Self) -> *const T {
        let ptr = this.ptr.as_ptr();
        core::mem::forget(this);
        ptr
    }

    pub unsafe fn from_raw(raw: *const T) -> Self {
        Self {
            ptr: NonNull::new_unchecked(raw as *mut T),
            alloc: P::VALUE,
            value: PhantomData,
        }
    }
}

impl<T: PoolArcAdapter, P: Pool<T>> Drop for PoolArc<T, P> {
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

impl<T: PoolArcAdapter, P: Pool<T> + Clone> Clone for PoolArc<T, P> {
    fn clone(&self) -> Self {
        self.get_count().fetch_add(1, Ordering::Relaxed);
        Self {
            ptr: self.ptr,
            alloc: P::clone(&self.alloc),
            value: PhantomData,
        }
    }
}

impl<T: PoolArcAdapter, P: Pool<T>> Deref for PoolArc<T, P> {
    type Target = T;
    fn deref(&self) -> &Self::Target {
        self.as_ref()
    }
}
