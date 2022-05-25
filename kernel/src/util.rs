use alloc::boxed::Box;
use core::any::Any;

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

pub type Private = Box<dyn Any + Send + Sync>;

#[macro_export]
macro_rules! private_impl {
    ($field:ident) => {
        pub fn private<T: 'static + Send + Sync>(&self) -> Option<&T> {
            let private: &Option<$crate::util::Private> = &self.$field;
            private.as_ref()?.downcast_ref()
        }

        pub fn private_mut<T: 'static + Send + Sync>(&mut self) -> Option<&mut T> {
            let private: &mut Option<$crate::util::Private> = &mut self.$field;
            private.as_mut()?.downcast_mut()
        }

        pub unsafe fn private_unchecked<T: 'static + Send + Sync>(&self) -> &T {
            let private: &Option<$crate::util::Private> = &self.$field;
            private.as_ref().unwrap_unchecked().downcast_ref_unchecked()
        }

        pub unsafe fn private_mut_unchecked<T: 'static + Send + Sync>(&mut self) -> &mut T {
            let private: &mut Option<$crate::util::Private> = &mut self.$field;
            private.as_mut().unwrap_unchecked().downcast_mut_unchecked()
        }
    };
}
pub use crate::private_impl;

#[macro_export]
macro_rules! private_project_impl {
    ($lock:ident : $lock_ty:ty => $field:ident) => {
        pub fn lock_private<T: 'static + Send + Sync>(
            &self,
        ) -> Option<chos_lib::sync::SpinlockGuardProject<'_, chos_lib::sync::NoSchedLockPolicy, $lock_ty, T>> {
            let guard = self.$lock.lock();
            guard.try_project(|value| value.$field.as_mut()?.downcast_mut())
        }

        pub unsafe fn lock_private_unchecked<T: 'static + Send + Sync>(
            &self,
        ) -> chos_lib::sync::SpinlockGuardProject<'_, chos_lib::sync::NoSchedLockPolicy, $lock_ty, T> {
            let guard = self.$lock.lock();
            guard.project(|value| value.$field.as_mut().unwrap_unchecked().downcast_mut_unchecked())
        }
    };
}
pub use crate::private_project_impl;
