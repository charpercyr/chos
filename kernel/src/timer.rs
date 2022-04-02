use alloc::borrow::Cow;
use alloc::collections::binary_heap::PeekMut;
use alloc::collections::BinaryHeap;
use alloc::sync::Arc;
use core::future::Future;
use core::mem::MaybeUninit;
use core::ops;
use core::pin::Pin;
use core::sync::atomic::{AtomicU64, Ordering};
use core::task::{Context, Poll, Waker};
use core::time::Duration;

use chos_config::timer::TICKS_HZ;
use chos_lib::arch::cache::CacheAligned;
use chos_lib::cpumask::Cpumask;
use chos_lib::int::ceil_divu64;
use chos_lib::sync::Spinlock;
use pin_project::pin_project;

use crate::arch::timer::arch_init_timer;
use crate::kmain::KernelArgs;
use crate::sched::ktask::{ktask_from_future, ktask_from_future_mask, KTask};
use crate::sched::schedule_tick;

pub const NS_PER_TICKS: u64 = 1_000_000_000 / TICKS_HZ;

struct TimerData {
    waker: Option<Waker>,
    ready: bool,
}

struct Timer {
    deadline: Instant,
    data: Spinlock<TimerData>,
}

struct TimerCmp(Arc<Timer>);

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

static TICKS: CacheAligned<AtomicU64> = CacheAligned::new(AtomicU64::new(0));

pub fn on_tick() {
    schedule_tick()
}

pub fn on_tick_main_cpu() {
    let ticks = TICKS.fetch_add(1, Ordering::Relaxed) + 1;
    {
        let mut timers = TIMERS.lock();
        let timers = unsafe { timers.assume_init_mut() };
        while let Some(tim) = timers.peek_mut() {
            if ticks >= tim.0.deadline.ticks() {
                let tim = PeekMut::pop(tim).0;
                {
                    let mut data = tim.data.lock();
                    data.ready = true;
                    if let Some(waker) = data.waker.take() {
                        waker.wake();
                    }
                }
            } else {
                break;
            }
        }
    }
    on_tick();
}

pub fn ticks() -> u64 {
    TICKS.load(Ordering::Relaxed)
}

const fn duration_to_ticks(d: Duration) -> u64 {
    ceil_divu64(d.as_nanos() as u64, NS_PER_TICKS)
}

pub fn init_timer(args: &KernelArgs) {
    *TIMERS.lock() = MaybeUninit::new(BinaryHeap::with_capacity(16));
    arch_init_timer(args);
}

#[pin_project]
pub struct Delay {
    timer: Arc<Timer>,
}

impl Future for Delay {
    type Output = ();
    fn poll(self: Pin<&mut Self>, cx: &mut Context) -> Poll<()> {
        let this = self.project();
        let mut data = this.timer.data.lock();
        if data.ready {
            Poll::Ready(())
        } else {
            data.waker = Some(cx.waker().clone());
            Poll::Pending
        }
    }
}

fn delay_timer(timer: Arc<Timer>) {
    let mut timers = TIMERS.lock();
    unsafe { timers.assume_init_mut().push(TimerCmp(timer)) };
}

pub fn delay_until(deadline: Instant) -> Delay {
    let timer = Arc::new(Timer {
        deadline,
        data: Spinlock::new(TimerData {
            waker: None,
            ready: false,
        }),
    });
    delay_timer(timer.clone());
    Delay { timer }
}

pub fn delay(d: Duration) -> Delay {
    delay_until(Instant::now() + d)
}

pub struct CancelToken<'a> {
    cancel: &'a mut bool,
}

impl CancelToken<'_> {
    pub fn cancel(self) {
        *self.cancel = true;
    }
}

#[must_use = "Must spawn the KTask"]
pub fn periodic_ktask(
    mut fun: impl FnMut(CancelToken) + Send + 'static,
    period: Duration,
    name: impl Into<Cow<'static, str>>,
) -> KTask {
    ktask_from_future(
        async move {
            let mut cancel = false;
            while !cancel {
                delay(period).await;
                fun(CancelToken {
                    cancel: &mut cancel,
                });
            }
        },
        name,
    )
}

#[must_use = "Must spawn the KTask"]
pub fn periodic_ktask_mask(
    mut fun: impl FnMut(CancelToken) + Send + 'static,
    period: Duration,
    name: impl Into<Cow<'static, str>>,
    mask: Cpumask,
) -> KTask {
    ktask_from_future_mask(
        async move {
            let mut cancel = false;
            while !cancel {
                delay(period).await;
                fun(CancelToken {
                    cancel: &mut cancel,
                });
            }
        },
        name,
        mask,
    )
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(transparent)]
pub struct Instant {
    ticks: u64,
}

impl Instant {
    pub const fn zero() -> Self {
        Self { ticks: 0 }
    }

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
