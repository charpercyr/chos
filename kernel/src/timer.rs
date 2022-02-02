use core::sync::atomic::{AtomicU64, Ordering};

use chos_lib::arch::cache::CacheAligned;

static TICKS: CacheAligned<AtomicU64> = CacheAligned::new(AtomicU64::new(0));

pub unsafe fn on_tick() {}

pub unsafe fn on_tick_main_cpu() {
    TICKS.fetch_add(1, Ordering::Relaxed);
}

pub fn ticks() -> u64 {
    TICKS.load(Ordering::Relaxed)
}
