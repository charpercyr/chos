use core::cell::UnsafeCell;
use core::hint::spin_loop;
use core::mem::{ManuallyDrop, MaybeUninit};
use core::ops::Deref;

use crate::init::ConstInit;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RawLazyState {
    First,
    Busy,
    Init,
}

pub trait RawLazy {
    fn is_init(&self) -> bool;
    unsafe fn access(&self) -> RawLazyState;
    unsafe fn mark_init(&self);
}

union LazyState<T> {
    uninit: fn() -> T,
    init: ManuallyDrop<T>,
}

pub struct Lazy<T, L: RawLazy> {
    lazy: L,
    state: UnsafeCell<LazyState<T>>,
}

impl<T, L: RawLazy> Lazy<T, L> {
    pub const fn new(init: fn() -> T) -> Self
    where
        L: ConstInit,
    {
        Self::new_with(init, L::INIT)
    }
    pub const fn new_with(init: fn() -> T, lazy: L) -> Self {
        Self {
            lazy,
            state: UnsafeCell::new(LazyState { uninit: init }),
        }
    }

    pub fn try_get(&self) -> Option<&T> {
        self.lazy.is_init().then(|| unsafe { self.get_value() })
    }

    pub fn get(&self) -> &T {
        unsafe {
            loop {
                match self.lazy.access() {
                    RawLazyState::First => {
                        self.init_value();
                        self.lazy.mark_init();
                        break;
                    }
                    RawLazyState::Busy => (),
                    RawLazyState::Init => break,
                }
                spin_loop();
            }
            self.get_value()
        }
    }

    unsafe fn init_value(&self) {
        let state = &mut *self.state.get();
        state.init = ManuallyDrop::new((state.uninit)());
    }

    unsafe fn get_value(&self) -> &T {
        let value = &(*self.state.get()).init;
        value
    }
}

impl<T, L: RawLazy> Deref for Lazy<T, L> {
    type Target = T;
    fn deref(&self) -> &Self::Target {
        self.get()
    }
}

unsafe impl<T: Send, L: RawLazy + Send> Send for Lazy<T, L> {}
unsafe impl<T: Sync, L: RawLazy + Sync> Sync for Lazy<T, L> {}

impl<T, L: RawLazy> Drop for Lazy<T, L> {
    fn drop(&mut self) {
        if self.lazy.is_init() {
            unsafe {
                ManuallyDrop::drop(&mut (*self.state.get()).init);
            }
        }
    }
}

pub struct OnceCell<T, L: RawLazy> {
    lazy: L,
    value: UnsafeCell<MaybeUninit<T>>,
}

impl<T, L: RawLazy> OnceCell<T, L> {
    pub const fn new_with(lazy: L) -> Self {
        Self {
            lazy,
            value: UnsafeCell::new(MaybeUninit::uninit()),
        }
    }

    pub const fn new() -> Self
    where
        L: ConstInit,
    {
        Self::new_with(L::INIT)
    }

    pub fn try_get(&self) -> Option<&T> {
        self.lazy.is_init().then(|| unsafe { self.get_value() })
    }

    pub fn get_or_set(&self, value: T) -> &T {
        self.get_or_init(|| value)
    }

    pub fn get_or_init<F: FnOnce() -> T>(&self, init: F) -> &T {
        unsafe {
            loop {
                match self.lazy.access() {
                    RawLazyState::First => {
                        self.set_value(init());
                        self.lazy.mark_init();
                        break;
                    }
                    RawLazyState::Busy => (),
                    RawLazyState::Init => break,
                }
                spin_loop();
            }
            self.get_value()
        }
    }

    pub fn force_set(cell: &Self, value: T) -> Result<(), T> {
        Self::force_init(cell, || value).map_err(|f| f())
    }

    pub fn force_init<F: FnOnce() -> T>(cell: &Self, init: F) -> Result<(), F> {
        unsafe {
            match cell.lazy.access() {
                RawLazyState::First => {
                    cell.set_value(init());
                    cell.lazy.mark_init();
                    Ok(())
                }
                RawLazyState::Busy | RawLazyState::Init => Err(init),
            }
        }
    }

    unsafe fn get_value(&self) -> &T {
        (*self.value.get()).assume_init_ref()
    }

    unsafe fn set_value(&self, value: T) {
        *self.value.get() = MaybeUninit::new(value);
    }

    pub unsafe fn as_mut(&mut self) -> Option<&mut T> {
        self.lazy
            .is_init()
            .then(move || self.as_mut_unchecked())
    }

    pub unsafe fn as_mut_unchecked(&mut self) -> &mut T {
        (*self.value.get()).assume_init_mut()
    }
}
unsafe impl<T: Send, L: RawLazy + Send> Send for OnceCell<T, L> {}
unsafe impl<T: Sync, L: RawLazy + Sync> Sync for OnceCell<T, L> {}

impl<T, L: RawLazy> Drop for OnceCell<T, L> {
    fn drop(&mut self) {
        if self.lazy.is_init() {
            unsafe { MaybeUninit::assume_init_drop(&mut *self.value.get()) };
        }
    }
}
