use core::cell::UnsafeCell;
use core::hint::spin_loop;
use core::intrinsics::likely;
use core::mem::{replace, MaybeUninit};
use core::ops::{Deref, DerefMut};

use super::{LockPolicy, NoIrqLockPolicy, NoOpLockPolicy, NoSchedLockPolicy};
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

    pub const fn new_with(value: T, lock: L) -> Self {
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
    pub fn lock_policy<P: LockPolicy>(&self) -> LockGuard<'_, P, L, T> {
        let meta = P::before_lock();
        self.lock.lock();
        LockGuard {
            lock: self,
            meta: MaybeUninit::new(meta),
        }
    }

    pub fn lock(&self) -> LockGuard<'_, NoSchedLockPolicy, L, T> {
        self.lock_policy()
    }

    pub fn lock_noirq(&self) -> LockGuard<'_, NoIrqLockPolicy, L, T> {
        self.lock_policy()
    }

    pub fn lock_nodisable(&self) -> LockGuard<'_, NoOpLockPolicy, L, T> {
        self.lock_policy()
    }

    pub fn try_lock_policy<P: LockPolicy>(&self) -> Option<LockGuard<'_, P, L, T>>
    where
        L: RawTryLock,
    {
        let meta = P::before_lock();
        if self.lock.try_lock() {
            Some(LockGuard {
                lock: self,
                meta: MaybeUninit::new(meta),
            })
        } else {
            P::after_unlock(meta);
            None
        }
    }

    pub fn try_lock(&self) -> Option<LockGuard<'_, NoSchedLockPolicy, L, T>>
    where
        L: RawTryLock,
    {
        self.try_lock_policy()
    }

    pub fn try_lock_noirq(&self) -> Option<LockGuard<'_, NoIrqLockPolicy, L, T>>
    where
        L: RawTryLock,
    {
        self.try_lock_policy()
    }

    pub fn try_lock_nodisable(&self) -> Option<LockGuard<'_, NoOpLockPolicy, L, T>>
    where
        L: RawTryLock,
    {
        self.try_lock_policy()
    }

    pub fn try_lock_tries_policy<P: LockPolicy>(
        &self,
        tries: usize,
    ) -> Option<LockGuard<'_, P, L, T>>
    where
        L: RawTryLock,
    {
        let meta = P::before_lock();
        if self.lock.try_lock_tries(tries) {
            Some(LockGuard {
                lock: self,
                meta: MaybeUninit::new(meta),
            })
        } else {
            P::after_unlock(meta);
            None
        }
    }

    pub fn try_lock_tries(&self, tries: usize) -> Option<LockGuard<'_, NoSchedLockPolicy, L, T>>
    where
        L: RawTryLock,
    {
        self.try_lock_tries_policy(tries)
    }

    pub fn try_lock_tries_noirq(&self, tries: usize) -> Option<LockGuard<'_, NoIrqLockPolicy, L, T>>
    where
        L: RawTryLock,
    {
        self.try_lock_tries_policy(tries)
    }

    pub fn try_lock_tries_nodisable(
        &self,
        tries: usize,
    ) -> Option<LockGuard<'_, NoOpLockPolicy, L, T>>
    where
        L: RawTryLock,
    {
        self.try_lock_tries_policy(tries)
    }

    pub fn get_mut(&mut self) -> &mut T {
        unsafe { &mut *self.value.get() }
    }

    pub fn get_ptr(&self) -> *mut T {
        self.value.get()
    }
}

impl<L: RawLock + ConstInit, T: ConstInit> ConstInit for Lock<L, T> {
    const INIT: Self = Self {
        lock: ConstInit::INIT,
        value: ConstInit::INIT,
    };
}

pub struct LockGuard<'a, P: LockPolicy, L: RawLock, T: ?Sized> {
    lock: &'a Lock<L, T>,
    meta: MaybeUninit<P::Metadata>,
}
impl<P: LockPolicy, L: RawLock, T: ?Sized> !Send for LockGuard<'_, P, L, T> {}
unsafe impl<P: LockPolicy, L: RawLock + Sync, T: ?Sized + Sync> Sync for LockGuard<'_, P, L, T> where
    P::Metadata: Sync
{
}

impl<P: LockPolicy, L: RawLock, T: ?Sized> Drop for LockGuard<'_, P, L, T> {
    fn drop(&mut self) {
        unsafe {
            self.lock.lock.unlock();
            P::after_unlock(replace(&mut self.meta, MaybeUninit::uninit()).assume_init());
        }
    }
}

impl<'a, P: LockPolicy, L: RawLock, T: ?Sized> LockGuard<'a, P, L, T> {
    pub fn get_ref(&self) -> &T {
        unsafe { &*self.lock.value.get() }
    }

    pub fn get_mut(&mut self) -> &mut T {
        unsafe { &mut *self.lock.value.get() }
    }

    pub fn project<V: ?Sized>(
        mut self,
        f: impl FnOnce(&mut T) -> &mut V,
    ) -> LockGuardProject<'a, P, L, T, V> {
        let value = f(self.get_mut()) as *mut V;
        LockGuardProject { guard: self, value }
    }

    pub fn try_project<R: ?Sized>(
        mut self,
        f: impl FnOnce(&mut T) -> Option<&mut R>,
    ) -> Option<LockGuardProject<'a, P, L, T, R>> {
        let value = f(self.get_mut())? as *mut R;
        Some(LockGuardProject { guard: self, value })
    }
}

impl<P: LockPolicy, L: RawLock, T: ?Sized> Deref for LockGuard<'_, P, L, T> {
    type Target = T;
    fn deref(&self) -> &Self::Target {
        self.get_ref()
    }
}
impl<P: LockPolicy, L: RawLock, T: ?Sized> DerefMut for LockGuard<'_, P, L, T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.get_mut()
    }
}

pub struct LockGuardProject<'a, P: LockPolicy, L: RawLock, T: ?Sized, V: ?Sized> {
    guard: LockGuard<'a, P, L, T>,
    value: *mut V,
}
impl<P: LockPolicy, L: RawLock, T: ?Sized, V: ?Sized> !Send for LockGuardProject<'_, P, L, T, V> {}
unsafe impl<P: LockPolicy, L: RawLock + Sync, T: ?Sized + Sync, V: ?Sized + Sync> Sync
    for LockGuardProject<'_, P, L, T, V>
where
    P::Metadata: Sync,
{
}

impl<'a, P: LockPolicy, L: RawLock, T: ?Sized, V: ?Sized> LockGuardProject<'a, P, L, T, V> {
    pub fn get_ref(&self) -> &V {
        unsafe { &*self.value }
    }

    pub fn get_mut(&mut self) -> &mut V {
        unsafe { &mut *self.value }
    }

    pub fn project<R: ?Sized>(
        mut self,
        f: impl FnOnce(&mut V) -> &mut R,
    ) -> LockGuardProject<'a, P, L, T, R> {
        let value = f(self.get_mut()) as *mut R;
        LockGuardProject {
            guard: self.guard,
            value,
        }
    }

    pub fn try_project<R: ?Sized>(
        mut self,
        f: impl FnOnce(&mut V) -> Option<&mut R>,
    ) -> Option<LockGuardProject<'a, P, L, T, R>> {
        let value = f(self.get_mut())? as *mut R;
        Some(LockGuardProject {
            guard: self.guard,
            value,
        })
    }
}

impl<P: LockPolicy, L: RawLock, T: ?Sized, V: ?Sized> Deref for LockGuardProject<'_, P, L, T, V> {
    type Target = V;
    fn deref(&self) -> &Self::Target {
        self.get_ref()
    }
}
impl<P: LockPolicy, L: RawLock, T: ?Sized, V: ?Sized> DerefMut
    for LockGuardProject<'_, P, L, T, V>
{
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.get_mut()
    }
}

fn foo() {
    struct Foo {
        value: u32,
    }
    use super::spin::lock::Spinlock;
    let lock = Spinlock::new(Foo { value: 0 });
    let guard = lock.lock();
    let mut guard = guard.project(|foo| &mut foo.value);
    *guard = 1;
}
