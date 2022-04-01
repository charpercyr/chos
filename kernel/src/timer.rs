use alloc::collections::BinaryHeap;
use core::mem::MaybeUninit;
use core::ops;
use core::sync::atomic::{AtomicU64, Ordering};
use core::time::Duration;

use chos_config::timer::TICKS_HZ;
use chos_lib::arch::cache::CacheAligned;
use chos_lib::int::ceil_divu64;
use chos_lib::sync::Spinlock;

use crate::arch::timer::arch_init_timer;
use crate::kmain::KernelArgs;
use crate::sched::ktask::KTask;
use crate::sched::schedule_tick;

pub const NS_PER_TICKS: u64 = 1_000_000_000 / TICKS_HZ;

static TICKS: CacheAligned<AtomicU64> = CacheAligned::new(AtomicU64::new(0));

struct Timer {
    deadline: u64,
    schedule: Schedule,
    task: KTask,
}

struct TimerCmp(Timer);

impl PartialEq for TimerCmp {
    fn eq(&self, other: &Self) -> bool {
        self.0.deadline == other.0.deadline
    }
}
impl Eq for TimerCmp {}

impl Ord for TimerCmp {
    fn cmp(&self, other: &Self) -> core::cmp::Ordering {
        self.0.deadline.cmp(&other.0.deadline).reverse()
    }
}
impl PartialOrd for TimerCmp {
    fn partial_cmp(&self, other: &Self) -> Option<core::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

static TIMERS: Spinlock<MaybeUninit<BinaryHeap<TimerCmp>>> = Spinlock::new(MaybeUninit::uninit());

pub fn on_tick() {
    schedule_tick()
}

pub fn on_tick_main_cpu() {
    let _ticks = TICKS.fetch_add(1, Ordering::Relaxed) + 1;
    on_tick();
}

pub fn ticks() -> u64 {
    TICKS.load(Ordering::Relaxed)
}

const fn duration_to_ticks(d: Duration) -> u64 {
    ceil_divu64(d.as_nanos() as u64, NS_PER_TICKS)
}

#[derive(Debug)]
pub enum Schedule {
    Periodic(Duration),
    OnShotIn(Duration),
    OnShotAt(Instant),
}

pub fn schedule_timer(schedule: Schedule, task: KTask) {
    let deadline = match schedule {
        Schedule::OnShotIn(d) | Schedule::Periodic(d) => ticks() + duration_to_ticks(d),
        Schedule::OnShotAt(i) => i.ticks,
    };
    let new_timer = TimerCmp(Timer {
        deadline,
        schedule,
        task,
    });
    let mut timers = TIMERS.lock_noirq();
    unsafe { timers.assume_init_mut().push(new_timer) };
}

pub fn init_timer(args: &KernelArgs) {
    *TIMERS.lock() = MaybeUninit::new(BinaryHeap::with_capacity(16));
    arch_init_timer(args);
}

#[derive(Clone, Copy, Debug)]
#[repr(transparent)]
pub struct Instant {
    ticks: u64,
}

impl Instant {
    pub fn now() -> Self {
        Self { ticks: ticks() }
    }

    pub fn ticks(self) -> u64 {
        self.ticks
    }
}

impl ops::Add<Duration> for Instant {
    type Output = Instant;
    fn add(self, rhs: Duration) -> Instant {
        Instant {
            ticks: self.ticks + duration_to_ticks(rhs),
        }
    }
}

impl ops::Add<Instant> for Duration {
    type Output = Instant;
    fn add(self, rhs: Instant) -> Instant {
        Instant {
            ticks: duration_to_ticks(self) + rhs.ticks,
        }
    }
}

impl ops::AddAssign<Duration> for Instant {
    fn add_assign(&mut self, rhs: Duration) {
        *self = *self + rhs;
    }
}

impl ops::Sub<Duration> for Instant {
    type Output = Instant;
    fn sub(self, rhs: Duration) -> Instant {
        Instant {
            ticks: self.ticks - duration_to_ticks(rhs),
        }
    }
}

impl ops::Sub<Instant> for Duration {
    type Output = Instant;
    fn sub(self, rhs: Instant) -> Instant {
        Instant {
            ticks: duration_to_ticks(self) - rhs.ticks,
        }
    }
}

impl ops::SubAssign<Duration> for Instant {
    fn sub_assign(&mut self, rhs: Duration) {
        *self = *self - rhs;
    }
}
