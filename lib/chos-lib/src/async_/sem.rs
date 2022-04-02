use core::future::Future;
use core::marker::PhantomPinned;
use core::pin::Pin;
use core::task::{Context, Poll, Waker};

use intrusive_collections::{intrusive_adapter, linked_list, UnsafeMut};

use crate::sync::Spinlock;

struct AsyncSemInner {
    count: usize,
    waiters: linked_list::LinkedList<WaiterAdapter>,
}

pub struct AsyncSem {
    inner: Spinlock<AsyncSemInner>,
}

impl AsyncSem {
    pub const fn zero() -> Self {
        Self::new(0)
    }
    pub const fn new(count: usize) -> Self {
        Self {
            inner: Spinlock::new(AsyncSemInner {
                count,
                waiters: linked_list::LinkedList::new(WaiterAdapter::NEW),
            }),
        }
    }

    pub fn wait_count(&self, count: usize) -> AsyncSemWaitFut<'_> {
        AsyncSemWaitFut {
            sem: self,
            waiter: Waiter {
                count,
                link: linked_list::AtomicLink::new(),
                waker: None,
            },
            pinned: PhantomPinned,
        }
    }

    pub fn wait(&self) -> AsyncSemWaitFut<'_> {
        self.wait_count(1)
    }

    pub fn try_wait_count(&self, count: usize) -> bool {
        let mut inner = self.inner.lock();
        if inner.count >= count {
            inner.count -= count;
            true
        } else {
            false
        }
    }

    pub fn try_wait(&self) -> bool {
        self.try_wait_count(1)
    }

    pub fn signal_count(&self, count: usize) {
        let mut woke_count = count;
        let mut inner = self.inner.lock_noirq();
        let mut cursor = inner.waiters.front_mut();
        while !cursor.is_null() && woke_count > 0 {
            let waiter = cursor.get().unwrap();
            if waiter.count <= count {
                let mut waiter = cursor.remove().unwrap();
                woke_count -= waiter.count;
                let waker = waiter.waker.take().unwrap();
                waker.wake();
            } else {
                cursor.move_next();
            }
        }
        inner.count += count;
    }

    pub fn signal(&self) {
        self.signal_count(1)
    }
}

struct Waiter {
    link: linked_list::AtomicLink,
    waker: Option<Waker>,
    count: usize,
}

#[must_use = "Future do nothing unless awaited"]
pub struct AsyncSemWaitFut<'sem> {
    waiter: Waiter,
    sem: &'sem AsyncSem,
    pinned: PhantomPinned,
}

intrusive_adapter!(WaiterAdapter = UnsafeMut<Waiter>: Waiter { link: linked_list::AtomicLink});

impl Future for AsyncSemWaitFut<'_> {
    type Output = ();

    fn poll(self: Pin<&mut Self>, cx: &mut Context) -> Poll<()> {
        let mut inner = self.sem.inner.lock();
        if inner.count >= self.waiter.count {
            inner.count -= self.waiter.count;
            Poll::Ready(())
        } else {
            let this = unsafe { self.get_unchecked_mut() };
            this.waiter.waker = Some(cx.waker().clone());
            unsafe {
                inner
                    .waiters
                    .push_back(UnsafeMut::from_raw(&mut this.waiter))
            }
            Poll::Pending
        }
    }
}

impl Drop for AsyncSemWaitFut<'_> {
    fn drop(&mut self) {
        let mut inner = self.sem.inner.lock_noirq();
        if self.waiter.link.is_linked() {
            unsafe { inner.waiters.cursor_mut_from_ptr(&self.waiter).remove() };
            debug_assert!(!self.waiter.link.is_linked());
        }
    }
}

#[cfg(test)]
mod tests {
    use alloc::sync::Arc;
    use core::ptr::null;
    use core::task::{RawWaker, RawWakerVTable};

    use super::*;

    static FAKE_VTABLE: RawWakerVTable = RawWakerVTable::new(
        |_| RawWaker::new(null(), &FAKE_VTABLE),
        |_| (),
        |_| (),
        |_| (),
    );
    fn fake_raw_waker() -> RawWaker {
        RawWaker::new(null(), &FAKE_VTABLE)
    }

    #[tokio::test]
    async fn signal_1() {
        let sem1 = Arc::new(AsyncSem::new(0));
        let sem2 = sem1.clone();
        let t1 = tokio::spawn(async move {
            sem1.signal();
        });
        let t2 = tokio::spawn(async move {
            sem2.wait().await;
        });
        t1.await.unwrap();
        t2.await.unwrap();
    }

    #[tokio::test]
    async fn signal_n() {
        let sem1 = Arc::new(AsyncSem::new(0));
        let sem2 = sem1.clone();
        let sem3 = sem1.clone();
        let t1 = tokio::spawn(async move {
            sem1.wait_count(2).await;
        });
        let t2 = tokio::spawn(async move {
            sem2.wait_count(1).await;
        });
        let t3 = tokio::spawn(async move {
            sem3.signal_count(3);
        });

        t1.await.unwrap();
        t2.await.unwrap();
        t3.await.unwrap();
    }

    #[test]
    fn try_wait() {
        let sem = AsyncSem::new(0);
        assert!(!sem.try_wait_count(1));
        sem.signal_count(3);
        assert!(sem.try_wait_count(1));
        assert!(sem.try_wait_count(2));
        assert!(!sem.try_wait_count(1));
    }

    #[test]
    fn drop_future() {
        // Making sure that dropping the future unlinks it from the list of wakers
        let waker = unsafe { Waker::from_raw(fake_raw_waker()) };
        let mut cx = Context::from_waker(&waker);
        let sem = AsyncSem::new(0);
        {
            let mut wait_fut = sem.wait();
            assert!(unsafe { Pin::new_unchecked(&mut wait_fut) }
                .poll(&mut cx)
                .is_pending());
            assert!(wait_fut.waiter.link.is_linked());
            assert!(!sem.inner.lock().waiters.front().is_null());
            // drop wait_fut
        }
        assert!(sem.inner.lock().waiters.front().is_null());
    }
}
