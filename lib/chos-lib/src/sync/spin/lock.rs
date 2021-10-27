use core::cell::Cell;
use core::hint::spin_loop;
use core::intrinsics::likely;
use core::sync::atomic::{AtomicBool, Ordering};

use crate::init::ConstInit;
use crate::sync::lock::{Lock, LockGuard, RawLock, RawTryLock};
use crate::sync::rwlock::{RWLock, RWLockReadGuard, RWLockWriteGuard, RawRWLock, RawTryRWLock};

pub struct RawSpinLock {
    lock: AtomicBool,
}

impl ConstInit for RawSpinLock {
    const INIT: Self = Self {
        lock: AtomicBool::new(false),
    };
}

unsafe impl RawLock for RawSpinLock {
    fn lock(&self) {
        loop {
            if likely(self.try_lock()) {
                return;
            }
            spin_loop();
        }
    }
    unsafe fn unlock(&self) {
        self.lock.store(false, Ordering::Release);
    }
}

unsafe impl RawTryLock for RawSpinLock {
    fn try_lock(&self) -> bool {
        self.lock
            .compare_exchange_weak(false, true, Ordering::Acquire, Ordering::Relaxed)
            .is_ok()
    }
}

pub type Spinlock<T> = Lock<RawSpinLock, T>;
pub type SpinlockGuard<'a, T> = LockGuard<'a, RawSpinLock, T>;

pub struct RawSpinRWLock {
    write_lock: RawSpinLock,
    lock: RawSpinLock,
    readers: Cell<usize>,
}
unsafe impl Send for RawSpinRWLock {}
unsafe impl Sync for RawSpinRWLock {}

impl ConstInit for RawSpinRWLock {
    const INIT: Self = Self {
        write_lock: RawSpinLock::INIT,
        lock: RawSpinLock::INIT,
        readers: Cell::new(0),
    };
}

unsafe impl RawRWLock for RawSpinRWLock {
    fn lock_read(&self) {
        self.lock.lock();
        if self.readers.get() == 0 {
            self.write_lock.lock();
        }
        self.readers.set(self.readers.get() + 1);
        unsafe { self.lock.unlock() };
    }
    unsafe fn unlock_read(&self) {
        self.lock.lock();
        if self.readers.get() == 1 {
            self.write_lock.unlock();
        }
        self.readers.set(self.readers.get() - 1);
        self.lock.unlock();
    }

    fn lock_write(&self) {
        self.write_lock.lock();
    }
    unsafe fn unlock_write(&self) {
        self.write_lock.unlock();
    }
}

unsafe impl RawTryRWLock for RawSpinRWLock {
    fn try_lock_read(&self) -> bool {
        if !self.lock.try_lock() {
            return false;
        }
        if self.readers.get() == 0 {
            if !self.write_lock.try_lock() {
                unsafe { self.lock.unlock() }
                return false;
            }
        }
        true
    }

    fn try_lock_write(&self) -> bool {
        self.write_lock.try_lock()
    }
}
pub type SpinRWLock<T> = RWLock<RawSpinRWLock, T>;
pub type SpinRWLockReadGuard<'a, T> = RWLockReadGuard<'a, RawSpinRWLock, T>;
pub type SpinRWLockWriteGuard<'a, T> = RWLockWriteGuard<'a, RawSpinRWLock, T>;
