use core::hint::spin_loop;
use core::sync::atomic::{AtomicU8, Ordering};

use crate::init::ConstInit;
use crate::sync::lazy::{OnceCell, RawLazy, RawLazyState};
use crate::sync::Lazy;

const STATE_UNINIT: u8 = 0;
const STATE_BUSY: u8 = 1;
const STATE_INIT: u8 = 2;

#[repr(transparent)]
pub struct RawSpinLazy {
    state: AtomicU8,
}

impl RawSpinLazy {
    pub const fn new() -> Self {
        Self {
            state: AtomicU8::new(STATE_UNINIT),
        }
    }
}
impl ConstInit for RawSpinLazy {
    const INIT: Self = Self::new();
}

impl RawLazy for RawSpinLazy {
    fn is_init(&self) -> bool {
        self.state.load(Ordering::Relaxed) == STATE_INIT
    }

    unsafe fn access(&self) -> RawLazyState {
        loop {
            let state = self.state.load(Ordering::Relaxed);
            if state == STATE_INIT {
                break RawLazyState::Init;
            } else if state == STATE_BUSY {
                break RawLazyState::Busy;
            } else if self
                .state
                .compare_exchange_weak(
                    STATE_UNINIT,
                    STATE_BUSY,
                    Ordering::Acquire,
                    Ordering::Relaxed,
                )
                .is_ok()
            {
                break RawLazyState::First;
            }
            spin_loop();
        }
    }

    unsafe fn mark_init(&self) {
        self.state.store(STATE_INIT, Ordering::Release);
    }
}

pub type SpinLazy<T> = Lazy<T, RawSpinLazy>;
pub type SpinOnceCell<T> = OnceCell<T, RawSpinLazy>;
