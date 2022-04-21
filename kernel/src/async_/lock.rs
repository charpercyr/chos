use core::cell::UnsafeCell;
use core::future::Future;
use core::ops::{Deref, DerefMut};
use core::pin::Pin;
use core::task::{Context, Poll};

use pin_project::pin_project;

use super::sem::AsyncSemWaitFut;
use super::AsyncSem;

pub struct AsyncLock<T: ?Sized> {
    sem: AsyncSem,
    value: UnsafeCell<T>,
}

impl<T: ?Sized> AsyncLock<T> {
    pub fn as_mut(&mut self) -> &mut T {
        unsafe { &mut *self.value.get() }
    }

    pub fn lock(&self) -> AsyncLockFut<'_, T> {
        AsyncLockFut {
            lock: self,
            fut: self.sem.wait(),
        }
    }

    pub fn try_lock(&self) -> Option<AsyncLockGuard<'_, T>> {
        self.sem.try_wait().then(|| AsyncLockGuard { lock: self })
    }
}

impl<T> AsyncLock<T> {
    pub const fn new(value: T) -> Self {
        Self {
            sem: AsyncSem::new(1),
            value: UnsafeCell::new(value),
        }
    }

    pub fn into_inner(self) -> T {
        self.value.into_inner()
    }
}

#[pin_project]
#[must_use = "Future do nothing unless awaited"]
pub struct AsyncLockFut<'lock, T: ?Sized> {
    lock: &'lock AsyncLock<T>,
    #[pin]
    fut: AsyncSemWaitFut<'lock>,
}

impl<'lock, T: ?Sized> Future for AsyncLockFut<'lock, T> {
    type Output = AsyncLockGuard<'lock, T>;
    fn poll(self: Pin<&mut Self>, cx: &mut Context) -> Poll<Self::Output> {
        let this = self.project();
        let fut = this.fut;
        let lock = this.lock;
        fut.poll(cx).map(move |_| AsyncLockGuard { lock })
    }
}

pub struct AsyncLockGuard<'lock, T: ?Sized> {
    lock: &'lock AsyncLock<T>,
}

impl<T: ?Sized> Drop for AsyncLockGuard<'_, T> {
    fn drop(&mut self) {
        self.lock.sem.signal()
    }
}

impl<T: ?Sized> AsyncLockGuard<'_, T> {
    pub fn as_ref(&self) -> &T {
        unsafe { &*self.lock.value.get() }
    }
    pub fn as_mut(&mut self) -> &mut T {
        unsafe { &mut *self.lock.value.get() }
    }
}

impl<T: ?Sized> Deref for AsyncLockGuard<'_, T> {
    type Target = T;
    fn deref(&self) -> &T {
        self.as_ref()
    }
}
impl<T: ?Sized> DerefMut for AsyncLockGuard<'_, T> {
    fn deref_mut(&mut self) -> &mut T {
        self.as_mut()
    }
}
