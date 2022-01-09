use core::hint::spin_loop;
use core::sync::atomic::{AtomicUsize, Ordering};

use crate::sync::sem::Sem;

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
    fn new_with_count(count: usize) -> Self {
        Self::new(count)
    }
    fn wait(&self) {
        loop {
            let count = loop {
                let count = self.count.load(Ordering::Relaxed);
                if count > 0 {
                    break count;
                }
                spin_loop();
            };
            if self
                .count
                .compare_exchange_weak(count, count - 1, Ordering::Acquire, Ordering::Relaxed)
                .is_ok()
            {
                break;
            }
        }
    }

    fn signal(&self) {
        self.count.fetch_add(1, Ordering::Release);
    }
}
