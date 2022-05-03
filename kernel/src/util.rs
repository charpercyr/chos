use core::any::Any;

use alloc::sync::Arc;

pub macro barrier($n:expr) {{
    static BARRIER: chos_lib::sync::SpinOnceCell<chos_lib::sync::SpinBarrier> =
        chos_lib::sync::SpinOnceCell::new();
    BARRIER
        .get_or_init(|| chos_lib::sync::SpinBarrier::new($n))
        .wait();
}}

pub macro do_once($body:expr) {{
    use core::sync::atomic::{AtomicBool, Ordering};
    static ONCE: AtomicBool = AtomicBool::new(false);
    if ONCE
        .compare_exchange(false, true, Ordering::AcqRel, Ordering::Relaxed)
        .is_ok()
    {
        $body;
    }
}}

pub type Private = Arc<dyn Any + Send + Sync>;

#[macro_export]
macro_rules! private_impl {
    ($field:ident) => {
        pub fn private<T: 'static + Send + Sync>(&self) -> Option<&T> {
            let private: &Option<$crate::util::Private> = &self.$field;
            private.as_ref()?.downcast_ref()
        }

        pub fn private_arc<T: 'static + Send + Sync>(&self) -> Option<alloc::sync::Arc<T>> {
            let private: &Option<$crate::util::Private> = &self.$field;
            let arc = private.as_ref()?.clone();
            alloc::sync::Arc::downcast(arc).ok()
        }

        pub unsafe fn private_unchecked<T: 'static + Send + Sync>(&self) -> &T {
            let private: &Option<$crate::util::Private> = &self.$field;
            private.as_ref().unwrap_unchecked().downcast_ref_unchecked()
        }

        pub unsafe fn private_arc_unchecked<T: 'static + Send + Sync>(&self) -> alloc::sync::Arc<T> {
            let private: &Option<$crate::util::Private> = &self.$field;
            let arc = private.as_ref().unwrap_unchecked().clone();
            alloc::sync::Arc::downcast(arc).unwrap_unchecked()
        }
    };
}
pub use crate::private_impl;
