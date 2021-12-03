use core::hint::spin_loop;
use core::sync::atomic::{AtomicUsize, Ordering};

pub struct SpinSem {
    count: AtomicUsize,
}

impl SpinSem {
    pub const fn new(count: usize) -> Self {
        Self {
            count: AtomicUsize::new(count),
        }
    }

    pub fn wait(&self) {
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

    pub fn signal(&self) {
        self.count.fetch_add(1, Ordering::Release);
    }
}
