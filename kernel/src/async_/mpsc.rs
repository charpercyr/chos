use alloc::sync::Arc;
use core::fmt::Debug;
use core::marker::PhantomData;
use core::pin::Pin;
use core::task::{Context, Poll, Waker};

use chos_lib::init::Init;
use chos_lib::queue::Queue;
use chos_lib::sync::Spinlock;
use futures::Stream;
use pin_project::pin_project;

struct Channel<T, Q: Queue<T>> {
    queue: Q,
    waker: Option<Waker>,
    closed: bool,
    data: PhantomData<T>,
}
type ChannelPtr<T, Q> = Arc<Spinlock<Channel<T, Q>>>;

pub struct Sender<T, Q: Queue<T>> {
    channel: ChannelPtr<T, Q>,
}

impl<T, Q: Queue<T>> Sender<T, Q> {
    pub fn try_send(&self, value: T) -> Result<(), Q::Error> {
        let mut channel = self.channel.lock();
        channel.queue.try_enqueue(value)?;
        if let Some(waker) = channel.waker.take() {
            waker.wake();
        }
        Ok(())
    }

    pub fn send(&self, value: T)
    where
        Q::Error: Debug,
    {
        self.try_send(value).unwrap()
    }

    pub fn send_replace(&self, value: T) -> Option<T> {
        let mut channel = self.channel.lock();
        channel.queue.enqueue_replace(value)
    }

    pub fn close(self) {
        drop(self)
    }
}

impl<T, Q: Queue<T>> Drop for Sender<T, Q> {
    fn drop(&mut self) {
        let mut channel = self.channel.lock();
        channel.closed = true;
        if let Some(waker) = channel.waker.take() {
            waker.wake();
        }
    }
}

#[pin_project]
pub struct Receiver<T, Q: Queue<T>> {
    channel: ChannelPtr<T, Q>,
}

impl<T, Q: Queue<T>> Stream for Receiver<T, Q> {
    type Item = T;
    fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<T>> {
        let this = self.project();
        let mut channel = this.channel.lock();
        if let Some(value) = channel.queue.dequeue() {
            Poll::Ready(Some(value))
        } else {
            if channel.closed {
                Poll::Ready(None)
            } else {
                channel.waker = Some(cx.waker().clone());
                Poll::Pending
            }
        }
    }
}

pub fn channel_with<T, Q: Queue<T>>(queue: Q) -> (Sender<T, Q>, Receiver<T, Q>) {
    let channel = Arc::new(Spinlock::new(Channel {
        queue,
        waker: None,
        data: PhantomData,
        closed: false,
    }));
    (
        Sender {
            channel: channel.clone(),
        },
        Receiver { channel },
    )
}

pub fn channel<T, Q: Queue<T>>() -> (Sender<T, Q>, Receiver<T, Q>)
where
    Q: Init,
{
    channel_with(Q::new())
}
