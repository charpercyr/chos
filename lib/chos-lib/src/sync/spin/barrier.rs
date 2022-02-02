use core::hint::spin_loop;
use core::sync::atomic::{AtomicUsize, Ordering, fence};

pub struct SpinBarrier {
    count: AtomicUsize,
    target: usize,
}

impl SpinBarrier {
    pub const fn new(count: usize) -> Self {
        Self {
            count: AtomicUsize::new(0),
            target: count,
        }
    }

    pub fn wait(&self) {
        let c = self.count.fetch_add(1, Ordering::Relaxed);
        if c < self.target - 1 {
            loop {
                if self.count.load(Ordering::Relaxed) >= self.target {
                    break;
                }
                spin_loop();
            }
        }
        fence(Ordering::AcqRel)
    }

    /// # Safety
    /// This can only be called if no other threads are waiting on this barrier
    pub unsafe fn reset(&mut self) {
        self.count.store(0, Ordering::Relaxed);
    }
}
