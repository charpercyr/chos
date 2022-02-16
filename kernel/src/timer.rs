use alloc::collections::binary_heap::PeekMut;
use alloc::collections::BinaryHeap;
use core::mem::MaybeUninit;
use core::sync::atomic::{AtomicU64, Ordering};
use core::time::Duration;

use chos_config::timer::TICKS_HZ;
use chos_lib::arch::cache::CacheAligned;
use chos_lib::int::ceil_divu128;
use chos_lib::log::todo_warn;
use chos_lib::sync::Spinlock;

use crate::arch::timer::arch_init_timer;
use crate::kmain::KernelArgs;
use crate::sched::schedule_tick;

pub const NS_PER_TICKS: u64 = 1_000_000_000 / TICKS_HZ;

static TICKS: CacheAligned<(AtomicU64, [u8; 64 - 8])> =
    CacheAligned::new((AtomicU64::new(0), [0; 64 - 8]));

#[derive(Debug)]
struct Timer {
    deadline: u64,
    schedule: Schedule,
    callback: fn(usize),
    data: usize,
}

#[derive(Debug)]
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
    let ticks = TICKS.0.fetch_add(1, Ordering::Relaxed) + 1;
    let mut timers = TIMERS.lock_nodisable();
    let timers = unsafe { timers.assume_init_mut() };
    while let Some(TimerCmp(mut timer)) = timers
        .peek_mut()
        .filter(|timer| ticks >= timer.0.deadline)
        .map(PeekMut::pop)
    {
        // TODO activate task in scheduler instead
        (timer.callback)(timer.data);
        if let Schedule::Periodic(d) = timer.schedule {
            timer.deadline = ticks + duration_to_ticks(d);
            timers.push(TimerCmp(timer));
        }
    }
    on_tick();
}

pub fn ticks() -> u64 {
    TICKS.0.load(Ordering::Relaxed)
}

const fn duration_to_ticks(d: Duration) -> u64 {
    ceil_divu128(d.as_nanos(), NS_PER_TICKS as u128) as u64
}

#[derive(Debug)]
pub enum Schedule {
    Periodic(Duration),
    OnShotIn(Duration),
}

pub fn schedule_timer(schedule: Schedule, callback: fn(usize), data: usize) {
    let deadline = match schedule {
        Schedule::OnShotIn(d) => ticks() + duration_to_ticks(d),
        Schedule::Periodic(d) => ticks() + duration_to_ticks(d),
    };
    let new_timer = TimerCmp(Timer {
        deadline,
        callback,
        data,
        schedule,
    });
    let mut timers = TIMERS.lock_noirq();
    unsafe { timers.assume_init_mut().push(new_timer) };
}

pub fn init_timer(args: &KernelArgs) {
    todo_warn!("activate task in scheduler instead");
    *TIMERS.lock() = MaybeUninit::new(BinaryHeap::with_capacity(16));
    arch_init_timer(args);
}
