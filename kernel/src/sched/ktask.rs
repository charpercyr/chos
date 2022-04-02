use alloc::borrow::Cow;
use alloc::boxed::Box;
use core::future::Future;
use core::pin::Pin;
use core::task::{Context, Poll, RawWaker, RawWakerVTable, Waker};

use chos_lib::log::debug;
use chos_lib::pool::{IArc, IArcAdapter, IArcCount};
use chos_lib::sync::Spinlock;
use intrusive_collections::linked_list;
use pin_project::pin_project;

use super::sync::SchedQueue;
use super::{Task, TaskArc, TaskOps, TaskRunningState};
use crate::cpumask::{self, Cpumask};
use crate::mm::slab::object_pool;
use crate::mm::virt::stack::Stack;
use crate::mm::{per_cpu, per_cpu_lazy, PerCpu};

mod private {
    pub trait Sealed {}
}

pub trait KTaskFn: 'static + Send {
    fn poll(self: Pin<&mut Self>, ctx: &mut Context) -> Poll<()>;
}

pub trait KTaskOutput: 'static + Send + Sync + private::Sealed {}
impl private::Sealed for () {}
impl KTaskOutput for () {}
impl private::Sealed for ! {}
impl KTaskOutput for ! {}

#[pin_project]
struct KTaskFnOnce<F>(Option<F>);
impl<R: KTaskOutput, F: FnOnce() -> R + Send + 'static> KTaskFn for KTaskFnOnce<F> {
    fn poll(self: Pin<&mut Self>, _: &mut Context) -> Poll<()> {
        let this = self.project();
        (this.0.take().expect("Should not have been called again"))();
        Poll::Ready(())
    }
}

#[pin_project]
struct KTaskFuture<F>(#[pin] F);
impl<F: Future<Output: KTaskOutput> + Send + 'static> KTaskFn for KTaskFuture<F> {
    fn poll(self: Pin<&mut Self>, ctx: &mut Context) -> Poll<()> {
        let this = self.project();
        this.0.poll(ctx).map(|_| ())
    }
}

static KTASK_OPS: TaskOps = TaskOps { wake: |_| {} };

per_cpu! {
    static mut ref KTASK_STACK: Option<Stack> = None;
}

per_cpu_lazy! {
    static mut ref KTASK_TASK: TaskArc = {
        let stack = KTASK_STACK.copy().expect("KTask stack not set");
        debug!("Using {:#x}-{:#x} for ktask stack", stack.range.start(), stack.range.end());
        Task::with_fn(
            stack,
            ktask_loop,
            "[ktask]",
            &KTASK_OPS,
            None,
        ).expect("KTask Task::new() should not fail")
    };
}

struct KTaskImpl {
    link: linked_list::AtomicLink,
    count: IArcCount,
    fun: Spinlock<Pin<Box<dyn KTaskFn>>>,
    name: Cow<'static, str>,
    mask: Cpumask,
}
impl IArcAdapter for KTaskImpl {
    fn count(&self) -> &IArcCount {
        &self.count
    }
}
object_pool!(struct KTaskImplPool : KTaskImpl);
type KTaskImplArc = IArc<KTaskImpl, KTaskImplPool>;

chos_lib::intrusive_adapter!(KTaskAdapter = KTaskImplArc : KTaskImpl { link: linked_list::AtomicLink });

static KTASK_QUEUE: SchedQueue<KTaskAdapter> = SchedQueue::new(KTaskAdapter::NEW);

static KTASK_WAKER_VTABLE: RawWakerVTable = {
    unsafe fn waker_clone(waker: *const ()) -> RawWaker {
        let waker_data = KTaskImplArc::from_raw(waker.cast());
        let new_waker_data = waker_data.clone();
        drop(KTaskImplArc::into_raw(waker_data));
        let new_ptr = KTaskImplArc::into_raw(new_waker_data);
        RawWaker::new(new_ptr.cast(), &KTASK_WAKER_VTABLE)
    }

    unsafe fn waker_wake(waker: *const ()) {
        let waker_data = KTaskImplArc::from_raw(waker.cast());
        KTASK_QUEUE.push(waker_data);
    }

    unsafe fn waker_wake_by_ref(waker: *const ()) {
        let waker_data = KTaskImplArc::from_raw(waker.cast());
        KTASK_QUEUE.push(waker_data.clone());
        drop(KTaskImplArc::into_raw(waker_data));
    }

    unsafe fn waker_drop(waker: *const ()) {
        let waker_data = KTaskImplArc::from_raw(waker.cast());
        drop(waker_data);
    }
    RawWakerVTable::new(waker_clone, waker_wake, waker_wake_by_ref, waker_drop)
};

fn ktask_loop() -> ! {
    let this_cpu_mask = cpumask::this_cpu();
    loop {
        if let Some(task) = KTASK_QUEUE.find_pop_wait(|ktask| ktask.mask.contains(this_cpu_mask)) {
            let task_clone_ptr = KTaskImplArc::into_raw(task.clone());
            let waker = unsafe {
                Waker::from_raw(RawWaker::new(task_clone_ptr.cast(), &KTASK_WAKER_VTABLE))
            };
            let mut ctx: Context = Context::from_waker(&waker);
            debug!("KTask run '{}'", task.name);
            if let Some(mut fun) = task.fun.try_lock_noirq() {
                match fun.as_mut().poll(&mut ctx) {
                    Poll::Pending => (), // Go to next task,
                    Poll::Ready(_) => {
                        drop(waker);
                        assert!(task.is_unique(), "This might bite me in the ass later.");
                    }
                }
            } else {
                panic!("KTask should never be locked except from this function");
            }
        }
    }
}

pub(super) fn find_next_task() -> Option<TaskArc> {
    KTASK_TASK.with(|task| {
        let running_state = IArc::get_mut(task)
            .map(|task| task.state.get_mut().running_state)
            .unwrap_or_else(|| {
                let state = task.state.lock();
                state.running_state
            });
        match running_state {
            TaskRunningState::Blocked => None,
            TaskRunningState::Ready => Some(task.clone()),
            TaskRunningState::Zombie => panic!("KTask should never exit"),
        }
    })
}

fn do_spawn(task: KTaskImplArc) {
    KTASK_QUEUE.push(task);
}

fn create_ktask(
    fun: impl KTaskFn,
    name: impl Into<Cow<'static, str>>,
    mask: Cpumask,
) -> KTaskImplArc {
    KTaskImplArc::new(KTaskImpl {
        link: linked_list::AtomicLink::new(),
        count: IArcCount::new(),
        fun: Spinlock::new(Box::pin(fun)),
        name: name.into(),
        mask,
    })
}

pub fn init_ktask_stack(stack: Stack) {
    KTASK_STACK.with(|ktask_stack| {
        if ktask_stack.is_some() {
            panic!("KTask stack already set");
        }
        *ktask_stack = Some(stack);
    });
}

#[repr(transparent)]
pub struct KTask(KTaskImplArc);

pub fn ktask_from_fn_mask<R: KTaskOutput>(
    fun: impl FnOnce() -> R + Send + 'static,
    name: impl Into<Cow<'static, str>>,
    mask: Cpumask,
) -> KTask {
    KTask(create_ktask(KTaskFnOnce(Some(fun)), name, mask))
}

pub fn ktask_from_fn<R: KTaskOutput>(
    fun: impl FnOnce() -> R + Send + 'static,
    name: impl Into<Cow<'static, str>>,
) -> KTask {
    ktask_from_fn_mask(fun, name, cpumask::all())
}

pub fn ktask_from_future_mask(
    fut: impl Future<Output: KTaskOutput> + Send + 'static,
    name: impl Into<Cow<'static, str>>,
    mask: Cpumask,
) -> KTask {
    KTask(create_ktask(KTaskFuture(fut), name, mask))
}

pub fn ktask_from_future(
    fut: impl Future<Output: KTaskOutput> + Send + 'static,
    name: impl Into<Cow<'static, str>>,
) -> KTask {
    ktask_from_future_mask(fut, name, cpumask::all())
}

pub fn spawn_mask<R: KTaskOutput>(
    fun: impl FnOnce() -> R + Send + 'static,
    name: impl Into<Cow<'static, str>>,
    mask: Cpumask,
) {
    do_spawn(ktask_from_fn_mask(fun, name, mask).0)
}

pub fn spawn<R: KTaskOutput>(
    f: impl FnOnce() -> R + Send + 'static,
    name: impl Into<Cow<'static, str>>,
) {
    spawn_mask(f, name, cpumask::all())
}

pub fn spawn_future_mask(
    fut: impl Future<Output: KTaskOutput> + Send + 'static,
    name: impl Into<Cow<'static, str>>,
    mask: Cpumask,
) {
    do_spawn(ktask_from_future_mask(fut, name, mask).0)
}

pub fn spawn_future(
    fut: impl Future<Output: KTaskOutput> + Send + 'static,
    name: impl Into<Cow<'static, str>>,
) {
    spawn_future_mask(fut, name, cpumask::all())
}

pub fn spawn_task(task: KTask) {
    do_spawn(task.0)
}
