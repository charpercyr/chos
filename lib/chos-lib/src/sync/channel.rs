#[cfg(feature = "alloc")]
pub mod oneshot {
    use alloc::sync::Arc;
    use core::cell::UnsafeCell;
    use core::mem::MaybeUninit;
    use core::ptr::replace;

    use crate::sync::Sem;

    struct ChannelData<T, S: Sem> {
        sem: S,
        data: UnsafeCell<MaybeUninit<T>>,
    }
    unsafe impl<T: Send, S: Sem + Send> Send for ChannelData<T, S> {}
    unsafe impl<T: Sync, S: Sem + Sync> Sync for ChannelData<T, S> {}

    pub struct Sender<T, S: Sem> {
        data: Arc<ChannelData<T, S>>,
    }

    impl<T, S: Sem> Sender<T, S> {
        pub fn send(self, value: T) {
            unsafe { *self.data.data.get() = MaybeUninit::new(value) };
            self.data.sem.signal();
        }
    }

    pub struct Receiver<T, S: Sem> {
        data: Arc<ChannelData<T, S>>,
    }

    impl<T, S: Sem> Receiver<T, S> {
        pub fn recv(self) -> T {
            self.data.sem.wait();
            unsafe { replace(self.data.data.get(), MaybeUninit::uninit()).assume_init() }
        }
    }

    pub fn channel<S: Sem, T>() -> (Sender<T, S>, Receiver<T, S>) {
        let data = Arc::new(ChannelData {
            data: UnsafeCell::new(MaybeUninit::uninit()),
            sem: S::new_with_count(0),
        });
        (Sender { data: data.clone() }, Receiver { data })
    }
}
