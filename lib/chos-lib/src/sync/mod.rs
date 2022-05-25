pub mod fake;
pub mod lazy;
pub mod lock;
pub mod rwlock;
pub mod sem;
pub mod spin;

pub use lazy::Lazy;
pub use lock::{Lock, LockGuard, RawLock, RawTryLock};
pub use rwlock::{RWLock, RWLockReadGuard, RWLockWriteGuard, RawRWLock, RawTryRWLock};
pub use sem::Sem;
pub use spin::barrier::SpinBarrier;
pub use spin::lazy::{SpinLazy, SpinOnceCell};
pub use spin::lock::{
    SpinRWLock, SpinRWLockReadGuard, SpinRWLockWriteGuard, Spinlock, SpinlockGuard,
    SpinlockGuardProject,
};
pub use spin::sem::SpinSem;

use crate::arch::intr::{disable_interrups_save, restore_interrupts, IntrStatus};

// I don't like this but it might be the easiest way to add nosched policy short of moving the lock code to the boot & kernel
extern "Rust" {
    fn __lock_disable_sched_save() -> u64;
    fn __lock_restore_sched(v: u64);
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
