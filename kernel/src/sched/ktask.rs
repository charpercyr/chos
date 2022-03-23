use alloc::borrow::Cow;
use core::future::Future;

use super::TaskArc;

pub fn find_next_task() -> Option<TaskArc> {
    None
}

pub fn spawn(_f: impl FnOnce() + Send + 'static, name: Cow<'static, str>) {
    todo!("Spawn function {}", name)
}

pub fn spawn_future(_f: impl Future<Output = ()> + Send + 'static, name: Cow<'static, str>) {
    todo!("Spawn future {}", name)
}
