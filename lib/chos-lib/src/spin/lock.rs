
use core::cell::UnsafeCell;
use core::ops::{Deref, DerefMut};

use super::RawLock;

pub struct Lock<T: ?Sized> {
    lock: RawLock,
    value: UnsafeCell<T>,
}

impl<T> Lock<T> {
    pub const fn new(value: T) -> Self {
        Self {
            lock: RawLock::new(),
            value: UnsafeCell::new(value),
        }
    }

    pub fn into_inner(self) -> T {
        self.value.into_inner()
    }
}

impl<T: ?Sized> Lock<T> {
    pub fn try_lock(&self) -> Option<LockGuard<'_, T>> {
        self.lock.try_lock().then(|| LockGuard {
            lock: self,
        })
    }

    pub fn try_lock_for(&self, n: usize) -> Option<LockGuard<'_, T>> {
        self.lock.try_lock_for(n).then(|| LockGuard {
            lock: self,
        })
    }

    pub fn lock(&self) -> LockGuard<'_, T> {
        self.lock.lock();
        LockGuard {
            lock: self,
        }
    }

    pub fn as_mut(&mut self) -> &mut T {
        unsafe { &mut *self.value.get() }
    }

    pub fn as_ptr(&self) -> *mut T {
        self.value.get()
    }
}

unsafe impl<T: ?Sized + Send> Send for Lock<T> {}
unsafe impl<T: ?Sized + Send> Sync for Lock<T> {}

#[must_use = "This structure will unlock the Lock as soon as it is dropped"]
pub struct LockGuard<'a, T: ?Sized> {
    lock: &'a Lock<T>,
}

impl<T: ?Sized> LockGuard<'_, T> {
    pub fn as_ref(&self) -> &T {
        unsafe { &*self.lock.value.get() }
    }

    pub fn as_mut(&mut self) -> &mut T {
        unsafe { &mut *self.lock.value.get() }
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

impl<T: ?Sized> Drop for LockGuard<'_, T> {
    fn drop(&mut self) {
        self.lock.lock.unlock()
    }
}
