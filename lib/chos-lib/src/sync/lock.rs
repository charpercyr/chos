use core::cell::UnsafeCell;
use core::hint::spin_loop;
use core::intrinsics::likely;
use core::ops::{Deref, DerefMut};

use crate::init::ConstInit;

pub unsafe trait RawLock {
    fn lock(&self);
    unsafe fn unlock(&self);
}

pub unsafe trait RawTryLock: RawLock {
    fn try_lock(&self) -> bool;
    fn try_lock_tries(&self, tries: usize) -> bool {
        for _ in 0..tries {
            if likely(self.try_lock()) {
                return true;
            }
            spin_loop();
        }
        false
    }
}

pub struct Lock<L: RawLock, T: ?Sized> {
    lock: L,
    value: UnsafeCell<T>,
}
unsafe impl<L: RawLock + Send, T: Send + ?Sized> Send for Lock<L, T> {}
unsafe impl<L: RawLock + Sync, T: Send + ?Sized> Sync for Lock<L, T> {}

impl<L: RawLock, T> Lock<L, T> {
    pub const fn new(value: T) -> Self
    where
        L: ConstInit,
    {
        Self {
            lock: L::INIT,
            value: UnsafeCell::new(value),
        }
    }

    pub const fn new_with_lock(value: T, lock: L) -> Self {
        Self {
            lock,
            value: UnsafeCell::new(value),
        }
    }

    pub fn into_inner(self) -> T {
        self.value.into_inner()
    }
}

impl<L: RawLock, T: ?Sized> Lock<L, T> {
    pub fn lock(&self) -> LockGuard<'_, L, T> {
        self.lock.lock();
        LockGuard { lock: self }
    }

    pub fn try_lock(&self) -> Option<LockGuard<'_, L, T>>
    where
        L: RawTryLock,
    {
        self.lock.try_lock().then(|| LockGuard { lock: self })
    }

    pub fn try_lock_tries(&self, tries: usize) -> Option<LockGuard<'_, L, T>>
    where
        L: RawTryLock,
    {
        self.lock
            .try_lock_tries(tries)
            .then(|| LockGuard { lock: self })
    }

    pub fn get_mut(&mut self) -> &mut T {
        unsafe { &mut *self.value.get() }
    }
}

impl<L: RawLock + ConstInit, T: ConstInit> ConstInit for Lock<L, T> {
    const INIT: Self = Self {
        lock: ConstInit::INIT,
        value: ConstInit::INIT,
    };
}

pub struct LockGuard<'a, L: RawLock, T: ?Sized> {
    lock: &'a Lock<L, T>,
}
impl<L: RawLock, T: ?Sized> !Send for LockGuard<'_, L, T> {}
unsafe impl<L: RawLock + Sync, T: ?Sized + Sync> Sync for LockGuard<'_, L, T> {}

impl<L: RawLock, T: ?Sized> Drop for LockGuard<'_, L, T> {
    fn drop(&mut self) {
        unsafe { self.lock.lock.unlock() }
    }
}

impl<L: RawLock, T: ?Sized> LockGuard<'_, L, T> {
    pub fn as_ref(&self) -> &T {
        unsafe { &*self.lock.value.get() }
    }

    pub fn as_mut(&mut self) -> &mut T {
        unsafe { &mut *self.lock.value.get() }
    }
}

impl<L: RawLock, T: ?Sized> Deref for LockGuard<'_, L, T> {
    type Target = T;
    fn deref(&self) -> &Self::Target {
        self.as_ref()
    }
}
impl<L: RawLock, T: ?Sized> DerefMut for LockGuard<'_, L, T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.as_mut()
    }
}
