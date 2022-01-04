use chos_lib::arch::intr::without_interrupts;

#[macro_export]
macro_rules! per_cpu {
    ($($(pub $(($($vis:tt)*))?)? static mut ref $name:ident: $ty:ty = $init:expr;)*) => {
        paste::item! {
            $(
                $(pub $(($($vis)*))*)* struct [<__PerCpu $name:camel>](());
                unsafe impl $crate::mm::PerCpu for [<__PerCpu $name:camel>] {
                    type Target = $ty;
                    #[inline]
                    fn get(&self) -> *mut Self::Target {
                        #[thread_local]
                        static mut VALUE: $ty = $init;
                        unsafe { &mut VALUE }
                    }
                }
                $(pub $(($($vis)*))*)* static $name: [<__PerCpu $name:camel>] = [<__PerCpu $name:camel>](());
            )*
        }
    };
}

#[macro_export]
macro_rules! per_cpu_lazy {
    ($($(pub $(($($vis:tt)*))?)? static mut ref $name:ident: $ty:ty = $init:expr;)*) => {
        paste::item! {
            $(
                $(pub $(($($vis)*))*)* struct [<__PerCpu $name:camel>](());
                unsafe impl $crate::mm::PerCpu for [<__PerCpu $name:camel>] {
                    type Target = $ty;
                    #[inline]
                    fn get(&self) -> *mut Self::Target {
                        unsafe fn get_inner() -> &'static mut Option<$ty> {
                            #[thread_local]
                            static mut VALUE: Option<$ty> = None;
                            &mut VALUE
                        }
                        let value = unsafe { get_inner() };
                        if value.is_none() {
                            *value = Some($init);
                        }
                        unsafe { value.as_mut().unwrap_unchecked() }
                    }
                }
                $(pub $(($($vis)*))*)* static $name: [<__PerCpu $name:camel>] = [<__PerCpu $name:camel>](());
            )*
        }
    };
}

#[macro_export]
macro_rules! per_cpu_with_all {
    (@call_fun [$body:expr]) => {
        $body
    };
    (@call_fun [$body:expr] $name:ident @ $val:expr, $($rest:tt)*) => {
        $val.with(move |$name| per_cpu_with_all!(@call_fun [$body] $($rest)*))
    };
    (($($name:ident @ $val:expr),* $(,)?) $body:expr) => {
        $crate::per_cpu_with_all!(@call_fun [$body] $($name @ $val,)*);
    };
}

pub unsafe trait PerCpu {
    type Target: ?Sized;
    fn get(&self) -> *mut Self::Target;

    fn with<R, F: FnOnce(&mut Self::Target) -> R>(&self, f: F) -> R {
        without_interrupts(move || unsafe { self.with_nosave_interrupts(f) })
    }

    unsafe fn with_nosave_interrupts<R, F: FnOnce(&mut Self::Target) -> R>(&self, f: F) -> R {
        f(&mut *self.get())
    }
}
