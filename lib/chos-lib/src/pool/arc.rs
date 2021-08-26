use core::alloc::{AllocError, Layout};
use core::fmt;
use core::marker::{PhantomData, Unpin};
use core::ops::Deref;
use core::ptr::NonNull;
use core::sync::atomic::{AtomicUsize, Ordering};

use super::Pool;

#[cfg(feature = "alloc")]
use alloc::alloc::{handle_alloc_error, Global};

pub struct IArcCount {
    count: AtomicUsize,
}

impl IArcCount {
    pub const INIT: Self = Self {
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
        let ptr = unsafe { alloc.allocate()?.cast() };
        unsafe { core::ptr::write(ptr.as_ptr(), value) };
        Ok(Self {
            ptr,
            alloc,
            value: PhantomData,
        })
    }

    pub fn new_in(value: T, alloc: P) -> Self {
        let r = Self::try_new_in(value, alloc);
        #[cfg(feature = "alloc")]
        {
            r.unwrap_or_else(|_| handle_alloc_error(Layout::new::<T>()))
        }
        #[cfg(not(feature = "alloc"))]
        {
            r.unwrap_or_else(|_| panic!("Could not allocate {} bytes", Layout::new::<T>().size()))
        }
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
}

#[cfg(feature = "alloc")]
impl<T: IArcAdapter> IArc<T, Global> {
    pub fn new(value: T) -> Self {
        Self::try_new_in(value, Global).unwrap_or_else(|_| handle_alloc_error(Layout::new::<T>()))
    }

    pub fn try_new(value: T) -> Result<Self, AllocError> {
        Self::try_new_in(value, Global)
    }

    pub fn into_raw(this: Self) -> *const T {
        let ptr = this.ptr.as_ptr();
        core::mem::forget(this);
        ptr
    }

    pub unsafe fn from_raw(raw: *const T) -> Self {
        Self {
            ptr: NonNull::new_unchecked(raw as *mut T),
            alloc: Global,
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
        self.as_ref()
    }
}

macro_rules! fmt {
    ($($fmt:ident),* $(,)?) => {
        $(
            impl<T: IArcAdapter + fmt::$fmt, P: Pool<T>> fmt::$fmt for IArc<T, P> {
                fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
                    fmt::$fmt::fmt(self.as_ref(), f)
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

impl<T: IArcAdapter, P: Unpin + Pool<T>> Unpin for IArc<T, P> {}
unsafe impl<T: IArcAdapter + Sync + Send, P: Pool<T> + Sync> Sync for IArc<T, P> {}
unsafe impl<T: IArcAdapter + Sync + Send, P: Pool<T> + Send> Send for IArc<T, P> {}

impl<T: IArcAdapter, P: Pool<T>> crate::intrusive::Pointer for IArc<T, P> {
    type Metadata = P;
    type Target = T;
    fn into_raw(this: Self) -> (*const Self::Target, Self::Metadata) {
        Self::into_raw_with_allocator(this)
    }
    unsafe fn from_raw(ptr: *const Self::Target, meta: Self::Metadata) -> Self {
        Self::from_raw_in(ptr, meta)
    }
}
