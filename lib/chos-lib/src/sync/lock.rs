use core::cell::UnsafeCell;
use core::hint::spin_loop;
use core::intrinsics::likely;
use core::mem::{MaybeUninit, replace};
use core::ops::{Deref, DerefMut};

use crate::arch::intr::{disable_interrups_save, IntrStatus, restore_interrupts};

use crate::init::ConstInit;

// I don't like this but it might be the easiest way to add nosched policy short of moving the lock code to the boot & kernel
extern "Rust" {
    fn __lock_disable_sched_save() -> u64;
    fn __lock_restore_sched(v: u64);
}

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

pub trait LockPolicy {
    type Metadata;
    fn before_lock() -> Self::Metadata;
    fn after_unlock(meta: Self::Metadata);
}

pub struct NoOpLockPolicy(());
impl LockPolicy for NoOpLockPolicy {
    type Metadata = ();
    fn before_lock() -> Self::Metadata {
        // Nothing
    }
    fn after_unlock(_meta: Self::Metadata) {
        // Nothing
    }
}

// TODO: Disable sched only
pub struct NoSchedLockPolicy(());
impl LockPolicy for NoSchedLockPolicy {
    type Metadata = u64;
    fn before_lock() -> Self::Metadata {
        unsafe { __lock_disable_sched_save() }
    }
    fn after_unlock(meta: Self::Metadata) {
        unsafe { __lock_restore_sched(meta) }
    }
}

pub struct NoIrqLockPolicy(());
impl LockPolicy for NoIrqLockPolicy {
    type Metadata = IntrStatus;
    fn before_lock() -> Self::Metadata {
        disable_interrups_save()
    }
    fn after_unlock(meta: Self::Metadata) {
        restore_interrupts(meta)
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
        LockGuard { lock: self, meta: MaybeUninit::new(meta) }
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
            Some(LockGuard { lock: self, meta: MaybeUninit::new(meta) })
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
            Some(LockGuard { lock: self, meta: MaybeUninit::new(meta) })
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

    pub fn try_lock_tries_nodisable(&self, tries: usize) -> Option<LockGuard<'_, NoOpLockPolicy, L, T>>
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

impl<P: LockPolicy, L: RawLock, T: ?Sized> LockGuard<'_, P, L, T> {
    pub fn get_ref(&self) -> &T {
        unsafe { &*self.lock.value.get() }
    }

    pub fn get_mut(&mut self) -> &mut T {
        unsafe { &mut *self.lock.value.get() }
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
