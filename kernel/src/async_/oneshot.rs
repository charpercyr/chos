use alloc::borrow::Cow;
use alloc::sync::Arc;
use core::future::Future;
use core::pin::Pin;
use core::task::{Context, Poll, Waker};

use chos_lib::sync::Spinlock;

use crate::sched::ktask::spawn_future;

struct Channel<T> {
    value: Option<T>,
    waker: Option<Waker>,
}
type ChannelPtr<T> = Arc<Spinlock<Channel<T>>>;

pub struct Sender<T> {
    channel: ChannelPtr<T>,
}

impl<T> Sender<T> {
    pub fn send(self, value: T) {
        let mut data = self.channel.lock();
        assert!(data.value.is_none(), "BUG: send() called twice?");
        data.value = Some(value);
        if let Some(waker) = data.waker.take() {
            waker.wake();
        }
    }

    pub fn send_with(self, f: impl FnOnce() -> T) {
        self.send(f())
    }

    pub fn send_with_future(self, f: impl Future<Output = T> + Send + 'static)
    where
        T: Send + 'static,
    {
        self.send_with_future_named(f, "sender::send_with_future ")
    }

    pub fn send_with_future_named(
        self,
        f: impl Future<Output = T> + Send + 'static,
        name: impl Into<Cow<'static, str>>,
    ) where
        T: Send + 'static,
    {
        spawn_future(
            async move {
                self.send(f.await);
            },
            name,
        )
    }
}

pub struct Receiver<T> {
    channel: ChannelPtr<T>,
}

impl<T> Future for Receiver<T> {
    type Output = T;

    fn poll(self: Pin<&mut Self>, cx: &mut Context) -> Poll<T> {
        let mut data = self.channel.lock();
        if let Some(value) = data.value.take() {
            Poll::Ready(value)
        } else {
            data.waker = Some(cx.waker().clone());
            Poll::Pending
        }
    }
}

pub fn channel<T>() -> (Sender<T>, Receiver<T>) {
    let channel = Arc::new(Spinlock::new(Channel {
        value: None,
        waker: None,
    }));
    (
        Sender {
            channel: channel.clone(),
        },
        Receiver { channel },
    )
}

pub macro call_with_sender {
    ($name:ident ($($args:expr),* $(,)?)) => {
        $crate::async_::call_with_sender!(($name)($($args,)*))
    },
    (($call:expr) ($($args:expr),* $(,)?)) => {{
        let (sender, recv) = $crate::async_::oneshot::channel();
        ($call)($($args,)* sender) as ();
        recv
    }},
}
