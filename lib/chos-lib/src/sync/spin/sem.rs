use core::hint::spin_loop;
use core::sync::atomic::{AtomicUsize, Ordering};

use crate::sync::sem::{Sem, TrySem};

pub struct SpinSem {
    count: AtomicUsize,
}

impl SpinSem {
    pub const fn new(count: usize) -> Self {
        Self {
            count: AtomicUsize::new(count),
        }
    }
}

impl Sem for SpinSem {
    fn with_count(count: usize) -> Self {
        Self::new(count)
    }

    fn wait_count(&self, count: usize) {
        while !self.try_wait_count(count) {
            spin_loop()
        }
    }

    fn signal_count(&self, count: usize) {
        self.count.fetch_add(count, Ordering::Release);
    }
}

impl TrySem for SpinSem {
    fn try_wait_count(&self, count: usize) -> bool {
        let sem_count = self.count.load(Ordering::Relaxed);
        if sem_count >= count {
            if self
                .count
                .compare_exchange(
                    sem_count,
                    sem_count - count,
                    Ordering::Acquire,
                    Ordering::Relaxed,
                )
                .is_ok()
            {
                return true;
            }
        }
        false
    }
}
