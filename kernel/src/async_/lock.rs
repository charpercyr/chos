use core::cell::UnsafeCell;
use core::future::Future;
use core::mem::{replace, MaybeUninit};
use core::ops::{Deref, DerefMut};
use core::pin::Pin;
use core::task::{Context, Poll};

use chos_lib::sync::{LockPolicy, NoOpLockPolicy};
use pin_project::pin_project;

use super::sem::AsyncSemWaitFut;
use super::AsyncSem;

pub struct AsyncLock<T: ?Sized> {
    sem: AsyncSem,
    value: UnsafeCell<T>,
}
unsafe impl<T: ?Sized + Send> Send for AsyncLock<T> {}
unsafe impl<T: ?Sized + Send> Sync for AsyncLock<T> {}

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

    fn try_lock_policy<P: LockPolicy>(&self) -> Option<AsyncLockGuard<'_, P, T>> {
        let meta = P::before_lock();
        match self.sem.try_wait() {
            true => Some(AsyncLockGuard {
                lock: self,
                meta: MaybeUninit::new(meta),
            }),
            false => {
                P::after_unlock(meta);
                None
            }
        }
    }

    pub fn try_lock(&self) -> Option<AsyncLockGuard<'_, NoOpLockPolicy, T>> {
        self.try_lock_policy()
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
    type Output = AsyncLockGuard<'lock, NoOpLockPolicy, T>;
    fn poll(self: Pin<&mut Self>, cx: &mut Context) -> Poll<Self::Output> {
        let this = self.project();
        let fut = this.fut;
        let lock = this.lock;
        fut.poll(cx).map(move |_| AsyncLockGuard {
            lock,
            meta: MaybeUninit::new(NoOpLockPolicy::before_lock()),
        })
    }
}

pub struct AsyncLockGuard<'lock, P: LockPolicy, T: ?Sized> {
    lock: &'lock AsyncLock<T>,
    meta: MaybeUninit<P::Metadata>,
}

impl<T: ?Sized, P: LockPolicy> Drop for AsyncLockGuard<'_, P, T> {
    fn drop(&mut self) {
        self.lock.sem.signal();
        unsafe { P::after_unlock(replace(&mut self.meta, MaybeUninit::uninit()).assume_init()) }
    }
}

impl<T: ?Sized, P: LockPolicy> AsyncLockGuard<'_, P, T> {
    pub fn as_ref(&self) -> &T {
        unsafe { &*self.lock.value.get() }
    }
    pub fn as_mut(&mut self) -> &mut T {
        unsafe { &mut *self.lock.value.get() }
    }
}

impl<T: ?Sized, P: LockPolicy> Deref for AsyncLockGuard<'_, P, T> {
    type Target = T;
    fn deref(&self) -> &T {
        self.as_ref()
    }
}
impl<T: ?Sized, P: LockPolicy> DerefMut for AsyncLockGuard<'_, P, T> {
    fn deref_mut(&mut self) -> &mut T {
        self.as_mut()
    }
}
