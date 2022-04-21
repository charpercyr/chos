use core::future::Future;
use core::marker::PhantomPinned;
use core::pin::Pin;
use core::ptr::NonNull;
use core::task::{Context, Poll, Waker};

use chos_lib::sync::Spinlock;
use intrusive_collections::{intrusive_adapter, linked_list, UnsafeMut};

pub(super) struct WaiterList {
    waiters: linked_list::LinkedList<WaiterAdapter>,
}

impl WaiterList {
    pub const fn new() -> Self {
        Self {
            waiters: linked_list::LinkedList::new(WaiterAdapter::NEW),
        }
    }

    pub fn add_to_waitlist(&mut self, waiter: Pin<&mut Waiter>, waker: Waker) {
        let waiter = unsafe { waiter.get_unchecked_mut() };
        waiter.waker = Some(waker);
        waiter.list = Some(NonNull::from(&*self));
        self.waiters
            .push_back(unsafe { UnsafeMut::from_raw(waiter) });
    }

    pub fn wake_count(&mut self, mut count: usize) {
        let mut cur = self.waiters.front_mut();
        while !cur.is_null() && count > 0 {
            if cur.get().unwrap().count <= count {
                let mut waiter = cur.remove().unwrap();
                count -= waiter.count;
                if let Some(waker) = waiter.waker.take() {
                    waker.wake();
                }
                assert!(waiter.list.is_some());
                waiter.list = None;
            } else {
                cur.move_next();
            }
        }
    }
}

pub struct Waiter {
    list: Option<NonNull<WaiterList>>,
    link: linked_list::AtomicLink,
    waker: Option<Waker>,
    count: usize,
    pinned: PhantomPinned,
}
unsafe impl Send for Waiter {}
unsafe impl Sync for Waiter {}

impl Waiter {
    pub const fn new(count: usize) -> Self {
        Self {
            list: None,
            link: linked_list::AtomicLink::new(),
            waker: None,
            count,
            pinned: PhantomPinned,
        }
    }
}

impl Drop for Waiter {
    fn drop(&mut self) {
        if self.link.is_linked() {
            let mut list = self.list.unwrap();
            unsafe {
                list.as_mut()
                    .waiters
                    .cursor_mut_from_ptr(self)
                    .remove()
                    .unwrap();
                assert!(!self.link.is_linked())
            }
        }
    }
}

struct AsyncSemInner {
    count: usize,
    waiters: WaiterList,
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
                waiters: WaiterList::new(),
            }),
        }
    }

    pub fn wait_count(&self, count: usize) -> AsyncSemWaitFut<'_> {
        AsyncSemWaitFut {
            sem: self,
            waiter: Waiter::new(count),
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
        let mut inner = self.inner.lock_noirq();
        inner.count += count;
        inner.waiters.wake_count(count);
    }

    pub fn signal(&self) {
        self.signal_count(1)
    }
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
            inner.waiters.add_to_waitlist(
                unsafe { Pin::new_unchecked(&mut this.waiter) },
                cx.waker().clone(),
            );
            Poll::Pending
        }
    }
}

#[cfg(test)]
mod tests {
    use alloc::sync::Arc;
    use core::ptr::null;
    use core::task::{RawWaker, RawWakerVTable};

    use super::*;

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
