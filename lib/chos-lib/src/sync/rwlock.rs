use core::cell::UnsafeCell;
use core::hint::spin_loop;
use core::intrinsics::likely;
use core::ops::{Deref, DerefMut};

use crate::init::ConstInit;

pub unsafe trait RawRWLock {
    fn lock_read(&self);
    unsafe fn unlock_read(&self);
    fn lock_write(&self);
    unsafe fn unlock_write(&self);
}

pub unsafe trait RawTryRWLock: RawRWLock {
    fn try_lock_read(&self) -> bool;
    fn try_lock_read_tries(&self, tries: usize) -> bool {
        for _ in 0..tries {
            if likely(self.try_lock_read()) {
                return true;
            }
            spin_loop();
        }
        false
    }

    fn try_lock_write(&self) -> bool;
    fn try_lock_write_tries(&self, tries: usize) -> bool {
        for _ in 0..tries {
            if likely(self.try_lock_write()) {
                return true;
            }
            spin_loop();
        }
        false
    }
}

pub struct RWLock<L: RawRWLock, T: ?Sized> {
    lock: L,
    value: UnsafeCell<T>,
}
unsafe impl<L: RawRWLock + Send, T: Send + ?Sized> Send for RWLock<L, T> {}
unsafe impl<L: RawRWLock + Sync, T: Send + ?Sized> Sync for RWLock<L, T> {}

impl<L: RawRWLock, T> RWLock<L, T> {
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

    pub fn lock_read(&self) -> RWLockReadGuard<'_, L, T> {
        self.lock.lock_read();
        RWLockReadGuard { lock: self }
    }

    pub fn lock_write(&self) -> RWLockWriteGuard<'_, L, T> {
        self.lock.lock_write();
        RWLockWriteGuard { lock: self }
    }

    pub fn try_lock_read(&self) -> Option<RWLockReadGuard<'_, L, T>>
    where
        L: RawTryRWLock,
    {
        self.lock
            .try_lock_read()
            .then(|| RWLockReadGuard { lock: self })
    }

    pub fn try_lock_read_tries(&self, tries: usize) -> Option<RWLockReadGuard<'_, L, T>>
    where
        L: RawTryRWLock,
    {
        self.lock
            .try_lock_read_tries(tries)
            .then(|| RWLockReadGuard { lock: self })
    }

    pub fn try_lock_write(&self) -> Option<RWLockWriteGuard<'_, L, T>>
    where
        L: RawTryRWLock,
    {
        self.lock
            .try_lock_write()
            .then(|| RWLockWriteGuard { lock: self })
    }

    pub fn try_lock_write_tries(&self, tries: usize) -> Option<RWLockWriteGuard<'_, L, T>>
    where
        L: RawTryRWLock,
    {
        self.lock
            .try_lock_write_tries(tries)
            .then(|| RWLockWriteGuard { lock: self })
    }

    pub fn get_mut(&mut self) -> &mut T {
        unsafe { &mut *self.value.get() }
    }

    pub fn get_ptr(&self) -> *mut T {
        self.value.get()
    }
}

pub struct RWLockReadGuard<'a, L: RawRWLock, T: ?Sized> {
    lock: &'a RWLock<L, T>,
}
impl<L: RawRWLock, T: ?Sized> !Send for RWLockReadGuard<'_, L, T> {}
unsafe impl<L: RawRWLock + Sync, T: ?Sized + Sync> Sync for RWLockReadGuard<'_, L, T> {}

impl<L: RawRWLock, T: ?Sized> Drop for RWLockReadGuard<'_, L, T> {
    fn drop(&mut self) {
        unsafe { self.lock.lock.unlock_read() }
    }
}

impl<L: RawRWLock, T: ?Sized> RWLockReadGuard<'_, L, T> {
    fn as_ref(&self) -> &T {
        unsafe { &*self.lock.value.get() }
    }
}
impl<L: RawRWLock, T: ?Sized> Deref for RWLockReadGuard<'_, L, T> {
    type Target = T;
    fn deref(&self) -> &Self::Target {
        self.as_ref()
    }
}

pub struct RWLockWriteGuard<'a, L: RawRWLock, T: ?Sized> {
    lock: &'a RWLock<L, T>,
}
impl<L: RawRWLock, T: ?Sized> !Send for RWLockWriteGuard<'_, L, T> {}
unsafe impl<L: RawRWLock + Sync, T: ?Sized + Sync> Sync for RWLockWriteGuard<'_, L, T> {}

impl<L: RawRWLock, T: ?Sized> Drop for RWLockWriteGuard<'_, L, T> {
    fn drop(&mut self) {
        unsafe { self.lock.lock.unlock_write() }
    }
}

impl<L: RawRWLock, T: ?Sized> RWLockWriteGuard<'_, L, T> {
    fn as_ref(&self) -> &T {
        unsafe { &*self.lock.value.get() }
    }
    fn as_mut(&mut self) -> &mut T {
        unsafe { &mut *self.lock.value.get() }
    }
}
impl<L: RawRWLock, T: ?Sized> Deref for RWLockWriteGuard<'_, L, T> {
    type Target = T;
    fn deref(&self) -> &Self::Target {
        self.as_ref()
    }
}
impl<L: RawRWLock, T: ?Sized> DerefMut for RWLockWriteGuard<'_, L, T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.as_mut()
    }
}
