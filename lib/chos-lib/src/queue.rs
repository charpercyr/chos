#[cfg(feature = "alloc")]
use alloc::boxed::Box;
#[cfg(feature = "alloc")]
use alloc::collections::VecDeque;
use core::fmt::Debug;
use core::mem::{replace, MaybeUninit};

use crate::init::ConstInit;
pub trait Queue<T> {
    type Error;

    fn try_enqueue(&mut self, value: T) -> Result<(), Self::Error>;
    fn enqueue_replace(&mut self, value: T) -> Option<T>;
    fn dequeue(&mut self) -> Option<T>;

    fn enqueue(&mut self, value: T)
    where
        Self::Error: Debug,
    {
        self.try_enqueue(value).unwrap()
    }
}

#[cfg(feature = "alloc")]
impl<T> Queue<T> for VecDeque<T> {
    type Error = !;

    fn try_enqueue(&mut self, value: T) -> Result<(), Self::Error> {
        Ok(self.push_back(value))
    }

    fn enqueue_replace(&mut self, value: T) -> Option<T> {
        unsafe { self.try_enqueue(value).unwrap_unchecked() } // Self::Error = !
        None
    }

    fn dequeue(&mut self) -> Option<T> {
        self.pop_front()
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
pub struct RingQueueFullError<T>(pub T);

struct RingQueueData {
    read_head: usize,
    write_head: usize,
    count: usize,
}

impl RingQueueData {
    pub const fn new() -> Self {
        Self {
            read_head: 0,
            write_head: 0,
            count: 0,
        }
    }

    unsafe fn try_enqueue<T>(
        &mut self,
        buf: &mut [MaybeUninit<T>],
        value: T,
    ) -> Result<(), RingQueueFullError<T>> {
        if self.count < buf.len() {
            buf[self.write_head] = MaybeUninit::new(value);
            self.write_head += 1;
            if self.write_head >= buf.len() {
                self.write_head = 0;
            }
            self.count += 1;
            Ok(())
        } else {
            Err(RingQueueFullError(value))
        }
    }

    unsafe fn enqueue_replace<T>(&mut self, buf: &mut [MaybeUninit<T>], value: T) -> Option<T> {
        if self.count < buf.len() {
            buf[self.write_head] = MaybeUninit::new(value);
            self.inc_read_head(buf);
            self.count += 1;
            None
        } else {
            let old = replace(&mut buf[self.write_head], MaybeUninit::new(value)).assume_init();
            self.inc_read_head(buf);
            self.inc_read_head(buf);
            Some(old)
        }
    }

    unsafe fn dequeue<T>(&mut self, buf: &mut [MaybeUninit<T>]) -> Option<T> {
        if self.count > 0 {
            let value = replace(&mut buf[self.read_head], MaybeUninit::uninit()).assume_init();
            self.inc_read_head(buf);
            self.count -= 1;
            Some(value)
        } else {
            None
        }
    }

    unsafe fn inc_write_head<T>(&mut self, buf: &mut [MaybeUninit<T>]) {
        self.write_head += 1;
        if self.write_head >= buf.len() {
            self.write_head = 0;
        }
    }

    unsafe fn inc_read_head<T>(&mut self, buf: &mut [MaybeUninit<T>]) {
        self.read_head += 1;
        if self.read_head >= buf.len() {
            self.read_head = 0;
        }
    }
}

#[cfg(feature = "alloc")]
pub struct HeapRingQueue<T> {
    buf: Box<[MaybeUninit<T>]>,
    data: RingQueueData,
}

#[cfg(feature = "alloc")]
impl<T> HeapRingQueue<T> {
    pub fn new(len: usize) -> Self {
        Self {
            buf: Box::new_uninit_slice(len),
            data: RingQueueData::new(),
        }
    }

    pub fn try_enqueue(&mut self, value: T) -> Result<(), RingQueueFullError<T>> {
        unsafe { self.data.try_enqueue(&mut self.buf, value) }
    }

    fn enqueue_replace(&mut self, value: T) -> Option<T> {
        unsafe { self.data.enqueue_replace(&mut self.buf, value) }
    }

    pub fn dequeue(&mut self) -> Option<T> {
        unsafe { self.data.dequeue(&mut self.buf) }
    }
}

#[cfg(feature = "alloc")]
impl<T> Queue<T> for HeapRingQueue<T> {
    type Error = RingQueueFullError<T>;

    fn try_enqueue(&mut self, value: T) -> Result<(), Self::Error> {
        Self::try_enqueue(self, value)
    }

    fn enqueue_replace(&mut self, value: T) -> Option<T> {
        Self::enqueue_replace(self, value)
    }

    fn dequeue(&mut self) -> Option<T> {
        Self::dequeue(self)
    }
}

pub struct SliceRingQueue<'a, T> {
    buf: &'a mut [MaybeUninit<T>],
    data: RingQueueData,
}

impl<'a, T> SliceRingQueue<'a, T> {
    pub fn new(buf: &'a mut [MaybeUninit<T>]) -> Self {
        Self {
            buf,
            data: RingQueueData::new(),
        }
    }

    pub fn try_enqueue(&mut self, value: T) -> Result<(), RingQueueFullError<T>> {
        unsafe { self.data.try_enqueue(self.buf, value) }
    }

    fn enqueue_replace(&mut self, value: T) -> Option<T> {
        unsafe { self.data.enqueue_replace(self.buf, value) }
    }

    pub fn dequeue(&mut self) -> Option<T> {
        unsafe { self.data.dequeue(self.buf) }
    }
}

impl<T> Queue<T> for SliceRingQueue<'_, T> {
    type Error = RingQueueFullError<T>;

    fn try_enqueue(&mut self, value: T) -> Result<(), Self::Error> {
        Self::try_enqueue(self, value)
    }

    fn enqueue_replace(&mut self, value: T) -> Option<T> {
        Self::enqueue_replace(self, value)
    }

    fn dequeue(&mut self) -> Option<T> {
        Self::dequeue(self)
    }
}

pub struct ArrayRingQueue<T, const N: usize> {
    buf: [MaybeUninit<T>; N],
    data: RingQueueData,
}

impl<T, const N: usize> ArrayRingQueue<T, N> {
    pub const fn new() -> Self {
        Self {
            buf: ConstInit::INIT,
            data: RingQueueData::new(),
        }
    }

    pub fn try_enqueue(&mut self, value: T) -> Result<(), RingQueueFullError<T>> {
        unsafe { self.data.try_enqueue(&mut self.buf, value) }
    }

    fn enqueue_replace(&mut self, value: T) -> Option<T> {
        unsafe { self.data.enqueue_replace(&mut self.buf, value) }
    }

    pub fn dequeue(&mut self) -> Option<T> {
        unsafe { self.data.dequeue(&mut self.buf) }
    }
}
impl<T, const N: usize> ConstInit for ArrayRingQueue<T, N> {
    const INIT: Self = Self::new();
}

impl<T, const N: usize> Queue<T> for ArrayRingQueue<T, N> {
    type Error = RingQueueFullError<T>;

    fn try_enqueue(&mut self, value: T) -> Result<(), Self::Error> {
        Self::try_enqueue(self, value)
    }

    fn enqueue_replace(&mut self, value: T) -> Option<T> {
        Self::enqueue_replace(self, value)
    }

    fn dequeue(&mut self) -> Option<T> {
        Self::dequeue(self)
    }
}
