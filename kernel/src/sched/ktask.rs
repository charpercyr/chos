use alloc::borrow::Cow;
use alloc::boxed::Box;
use core::future::Future;
use core::pin::Pin;
use core::ptr::null;
use core::task::{Context, Poll, RawWaker, RawWakerVTable, Waker};

use chos_config::arch::mm::stack;
use chos_lib::log::debug;
use chos_lib::pool::{IArc, PoolBox};
use intrusive_collections::linked_list;
use pin_project::pin_project;

use super::sync::SchedQueue;
use super::{Task, TaskArc, TaskOps, TaskRunningState};
use crate::cpumask::{self, Cpumask};
use crate::mm::slab::object_pool;
use crate::mm::virt::stack::alloc_kernel_stack;
use crate::mm::{per_cpu_lazy, PerCpu};

mod private {
    pub trait Sealed {}
}

pub trait KTaskFn: 'static + Send {
    fn poll(self: Pin<&mut Self>, ctx: &mut Context) -> Poll<()>;
}

pub trait KTaskOutput: private::Sealed {}
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

per_cpu_lazy! {
    static mut ref KTASK_TASK: TaskArc = {
        let stack = alloc_kernel_stack(stack::KERNEL_STACK_PAGE_ORDER).expect("Stack alloc should not fail");
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
    fun: Pin<Box<dyn KTaskFn>>,
    name: Cow<'static, str>,
    mask: Cpumask,
}
object_pool!(struct KTaskImplPool : KTaskImpl);
type KTaskImplBox = PoolBox<KTaskImpl, KTaskImplPool>;

chos_lib::intrusive_adapter!(KTaskAdapter = KTaskImplBox : KTaskImpl { link: linked_list::AtomicLink });

static KTASK_QUEUE: SchedQueue<KTaskAdapter> = SchedQueue::new(KTaskAdapter::NEW);

static KTASK_WAKER_VTABLE: RawWakerVTable =
    RawWakerVTable::new(waker_clone, waker_wake, waker_wake_by_ref, waker_drop);

unsafe fn waker_clone(_waker: *const ()) -> RawWaker {
    todo!()
}

unsafe fn waker_wake(_waker: *const ()) {
    todo!()
}

unsafe fn waker_wake_by_ref(_waker: *const ()) {
    todo!()
}

unsafe fn waker_drop(_waker: *const ()) {
    todo!()
}

fn ktask_loop() -> ! {
    let waker = unsafe { Waker::from_raw(RawWaker::new(null(), &KTASK_WAKER_VTABLE)) };
    let this_cpu_mask = cpumask::this_cpu();
    loop {
        if let Some(mut task) =
            KTASK_QUEUE.find_pop_wait(|ktask| ktask.mask.contains(this_cpu_mask))
        {
            let mut ctx: Context = Context::from_waker(&waker);
            match task.fun.as_mut().poll(&mut ctx) {
                Poll::Pending => todo!(),
                Poll::Ready(_) => (),
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

fn do_spawn(task: KTaskImplBox) {
    KTASK_QUEUE.push(task);
}

fn create_ktask(
    fun: impl KTaskFn,
    name: impl Into<Cow<'static, str>>,
    mask: Cpumask,
) -> KTaskImplBox {
    KTaskImplBox::new(KTaskImpl {
        link: linked_list::AtomicLink::new(),
        fun: Box::pin(fun),
        name: name.into(),
        mask,
    })
}

#[repr(transparent)]
pub struct KTask(KTaskImplBox);

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
