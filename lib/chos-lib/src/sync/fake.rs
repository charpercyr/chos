use super::lock::{RawLock, RawTryLock};
use super::rwlock::{RawRWLock, RawTryRWLock};

pub struct FakeLock(());

impl FakeLock {
    pub const unsafe fn new() -> Self {
        Self(())
    }
}

unsafe impl RawLock for FakeLock {
    fn lock(&self) {
        // Nothing
    }
    unsafe fn unlock(&self) {
        // Nothing
    }
}

unsafe impl RawTryLock for FakeLock {
    fn try_lock(&self) -> bool {
        true
    }
}

unsafe impl RawRWLock for FakeLock {
    fn lock_read(&self) {
        // Nothing
    }
    unsafe fn unlock_read(&self) {
        // Nothing
    }
    fn lock_write(&self) {
        // Nothing
    }
    unsafe fn unlock_write(&self) {
        // Nothing
    }
}

unsafe impl RawTryRWLock for FakeLock {
    fn try_lock_read(&self) -> bool {
        true
    }
    fn try_lock_write(&self) -> bool {
        true
    }
}
