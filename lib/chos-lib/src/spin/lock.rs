
use core::cell::UnsafeCell;
use core::fmt;
use core::hint::spin_loop;
use core::intrinsics::likely;
use core::ops::{Deref, DerefMut};
use core::sync::atomic::{AtomicBool, Ordering};

pub struct RawSpinLock {
    lock: AtomicBool,
}

impl RawSpinLock {
    pub const fn new() -> Self {
        Self {
            lock: AtomicBool::new(false),
        }
    }

    #[must_use = "The lock might not have been taken"]
    #[inline]
    pub unsafe fn try_lock(&self) -> bool {
        self.lock.compare_exchange_weak(false, true, Ordering::Acquire, Ordering::Relaxed).is_ok()
    }

    #[must_use = "The lock might not have been taken"]
    #[inline]
    pub unsafe fn try_lock_tries(&self, tries: usize) -> bool {
        for _ in 0..tries {
            if likely(self.try_lock()) {
                return true;
            }
            spin_loop();
        }
        false
    }

    #[inline]
    pub unsafe fn lock(&self) {
        loop {
            if likely(self.try_lock()) {
                return;
            }
            spin_loop();
        }
    }

    #[inline]
    pub unsafe fn unlock(&self) {
        self.lock.store(false, Ordering::Release);
    }
}

impl fmt::Debug for RawSpinLock {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("RawLock")
    }
}

pub unsafe trait RawLock {
    fn lock(&self);
    unsafe fn unlock(&self);
}

pub unsafe trait RawTryLock {
    fn try_lock(&self) -> bool;
    fn try_lock_tries(&self, tries: usize) -> bool {
        for _ in 0..tries {
            if likely(self.try_lock()) {
                return true;
            }
        }
        false
    }
}

pub struct Lock<T: ?Sized> {
    lock: RawSpinLock,
    value: UnsafeCell<T>,
}

impl<T> Lock<T> {
    pub const fn new(value: T) -> Self {
        Self {
            lock: RawSpinLock::new(),
            value: UnsafeCell::new(value),
        }
    }

    pub fn into_inner(self) -> T {
        self.value.into_inner()
    }
}

impl<T: ?Sized> Lock<T> {
    pub fn as_mut(&mut self) -> &mut T {
        unsafe { &mut *self.value.get() }
    }

    pub fn try_lock(&self) -> Option<LockGuard<'_, T>> {
        unsafe { self.lock.try_lock() }.then(|| LockGuard { lock: self })
    }

    pub fn try_lock_tries(&self, tries: usize) -> Option<LockGuard<'_, T>> {
        unsafe { self.lock.try_lock_tries(tries) }.then(|| LockGuard { lock: self })
    }

    pub fn lock(&self) -> LockGuard<'_, T> {
        unsafe { self.lock.lock() };
        LockGuard { lock: self }
    }
}

unsafe impl<T: ?Sized + Send> Send for Lock<T> {}
unsafe impl<T: ?Sized + Send> Sync for Lock<T> {}

pub struct LockGuard<'a, T: ?Sized> {
    lock: &'a Lock<T>,
}

impl<T: ?Sized> LockGuard<'_, T> {
    pub fn as_ref(&self) -> &T {
        unsafe { & *self.lock.value.get() }
    }

    pub fn as_mut(&mut self) -> &mut T {
        unsafe { &mut *self.lock.value.get() }
    }
}

impl<T: ?Sized> Drop for LockGuard<'_, T> {
    fn drop(&mut self) {
        unsafe { self.lock.lock.unlock() }
    }
}

impl<T: ?Sized> Deref for LockGuard<'_, T> {
    type Target = T;
    fn deref(&self) -> &Self::Target {
        self.as_ref()
    }
}
impl<T: ?Sized> DerefMut for LockGuard<'_, T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.as_mut()
    }
}