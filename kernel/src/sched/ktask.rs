use alloc::borrow::Cow;
use core::future::Future;
use core::pin::Pin;
use core::task::{Context, Poll};

use chos_config::arch::mm::stack;
use chos_lib::log::debug;
use pin_project::pin_project;

use super::{Task, TaskArc, TaskOps};
use crate::cpumask::{self, Cpumask};
use crate::mm::per_cpu_lazy;
use crate::mm::virt::stack::alloc_kernel_stack;

mod private {
    pub trait Sealed {}
}

struct KTaskContext {}

impl KTaskContext {
    fn context(&self) -> Context {
        todo!("Create async Context")
    }
}

trait KTaskFn: 'static + Send {
    fn poll(self: Pin<&mut Self>, task: KTaskContext) -> Poll<()>;
}

pub trait KTaskOutput: private::Sealed {}
impl private::Sealed for () {}
impl KTaskOutput for () {}
impl private::Sealed for ! {}
impl KTaskOutput for ! {}

#[pin_project]
struct KTaskFnOnce<F>(Option<F>);
impl<R: KTaskOutput, F: FnOnce() -> R + Send + 'static> KTaskFn for KTaskFnOnce<F> {
    fn poll(self: Pin<&mut Self>, _: KTaskContext) -> Poll<()> {
        let this = self.project();
        (this.0.take().expect("Should not have been called again"))();
        Poll::Ready(())
    }
}

#[pin_project]
struct KTaskFuture<F>(#[pin] F);
impl<F: Future<Output: KTaskOutput> + Send + 'static> KTaskFn for KTaskFuture<F> {
    fn poll(self: Pin<&mut Self>, task: KTaskContext) -> Poll<()> {
        let this = self.project();
        let mut ctx = task.context();
        this.0.poll(&mut ctx).map(|_| ())
    }
}

static KTASK_OPS: TaskOps = TaskOps { wake: |_| todo!() };

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

fn ktask_loop() -> ! {
    todo!()
}

pub(super) fn find_next_task() -> Option<TaskArc> {
    None
}

fn do_spawn(fun: impl KTaskFn, name: impl Into<Cow<'static, str>>, mask: Cpumask) {
    todo!()
}

pub fn spawn_mask<R: KTaskOutput>(
    f: impl FnOnce() -> R + Send + 'static,
    name: impl Into<Cow<'static, str>>,
    mask: Cpumask,
) {
    do_spawn(KTaskFnOnce(Some(f)), name, mask)
}

pub fn spawn<R: KTaskOutput>(
    f: impl FnOnce() -> R + Send + 'static,
    name: impl Into<Cow<'static, str>>,
) {
    spawn_mask(f, name, cpumask::all())
}

pub fn spawn_future_mask(
    f: impl Future<Output: KTaskOutput> + Send + 'static,
    name: impl Into<Cow<'static, str>>,
    mask: Cpumask,
) {
    do_spawn(KTaskFuture(f), name, mask)
}

pub fn spawn_future(
    f: impl Future<Output: KTaskOutput> + Send + 'static,
    name: impl Into<Cow<'static, str>>,
) {
    spawn_future_mask(f, name, cpumask::all())
}
