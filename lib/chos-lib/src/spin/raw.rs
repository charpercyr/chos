
use core::hint::spin_loop;
use core::sync::atomic::{AtomicBool, Ordering};

#[repr(transparent)]
pub struct RawLock {
    lock: AtomicBool,
}

impl RawLock {
    pub const fn new() -> Self {
        Self {
            lock: AtomicBool::new(false),
        }
    }

    #[inline(always)]
    pub fn try_lock(&self) -> bool {
        self.lock
            .compare_exchange_weak(false, true, Ordering::Acquire, Ordering::Relaxed)
            .is_ok()
    }

    #[inline(always)]
    pub fn try_lock_for(&self, n: usize) -> bool {
        for _ in 0..n {
            if self.try_lock() {
                return true;
            }
            spin_loop();
        }
        false
    }

    #[inline(always)]
    pub fn lock(&self) {
        while !self.try_lock() {
            spin_loop();
        }
    }

    #[inline(always)]
    pub fn unlock(&self) {
        self.lock.store(false, Ordering::Release);
    }
}
