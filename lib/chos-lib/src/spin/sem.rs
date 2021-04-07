use core::hint::spin_loop;
use core::sync::atomic::{AtomicUsize, Ordering};

pub struct Sem {
    count: AtomicUsize,
}

impl Sem {
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
            if let Ok(_) = self.count.compare_exchange_weak(
                count,
                count - 1,
                Ordering::Acquire,
                Ordering::Relaxed,
            ) {
                break;
            }
        }
    }

    pub fn signal(&self) {
        self.count.fetch_add(1, Ordering::Release);
    }
}
