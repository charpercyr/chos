use core::hint::spin_loop;
use core::intrinsics::likely;
use core::sync::atomic::{AtomicBool, Ordering};

use super::super::lock::{Lock, LockGuard, RawLock, RawTryLock};

pub struct RawSpinLock {
    lock: AtomicBool,
}

unsafe impl RawLock for RawSpinLock {
    const INIT: Self = Self {
        lock: AtomicBool::new(false),
    };

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
