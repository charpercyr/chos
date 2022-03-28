use chos_lib::arch::intr::without_interrupts;
use chos_lib::mm::{PAddr, VAddr};

use super::virt::paddr_of;
use crate::arch::mm::per_cpu::{arch_this_cpu_info, per_cpu_base, per_cpu_base_for};

pub macro per_cpu ($($(pub $(($($vis:tt)*))?)? static mut ref $name:ident: $ty:ty = $init:expr;)*) {
    paste::item! {
        $(
            $(pub $(($($vis)*))*)* struct [<__PerCpu $name:camel>](());
            unsafe impl $crate::mm::PerCpu for [<__PerCpu $name:camel>] {
                type This = Self;
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
}

pub macro per_cpu_lazy ($($(pub $(($($vis:tt)*))?)? static mut ref $name:ident: $ty:ty = $init:expr;)*) {
    paste::item! {
        $(
            $(pub $(($($vis)*))*)* struct [<__PerCpu $name:camel>](());
            unsafe impl $crate::mm::PerCpu for [<__PerCpu $name:camel>] {
                type This = Self;
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
}

#[macro_export]
pub macro per_cpu_with_all {
    (@call_fun [$body:expr]) => {
        $body
    },
    (@call_fun [$body:expr] $name:ident @ $val:expr, $($rest:tt)*) => {
        $val.with(move |$name| per_cpu_with_all!(@call_fun [$body] $($rest)*))
    },
    (($($name:ident @ $val:expr),* $(,)?) $body:expr) => {
        $crate::per_cpu_with_all!(@call_fun [$body] $($name @ $val,)*);
    },
}

pub unsafe trait PerCpu {
    type This;
    type Target: 'static + ?Sized;
    fn get(&self) -> *mut Self::Target;

    unsafe fn get_ref<'a>(&self) -> &'a Self::Target {
        &*self.get()
    }
    unsafe fn get_mut<'a>(&self) -> &'a mut Self::Target {
        &mut *self.get()
    }

    fn paddr(&self) -> PAddr {
        unsafe {
            let (addr, _) = self.get().to_raw_parts();
            let vaddr = VAddr::new_unchecked(addr as u64);
            paddr_of(vaddr, super::virt::MemoryRegionType::PerCpu).expect("PAddr should be valid")
        }
    }

    unsafe fn with_static<R, F: FnOnce(&'static mut Self::Target) -> R>(&self, f: F) -> R {
        without_interrupts(move || self.with_static_nosave_interrupts(f))
    }

    fn with<R, F: FnOnce(&mut Self::Target) -> R>(&self, f: F) -> R {
        without_interrupts(move || unsafe { self.with_nosave_interrupts(f) })
    }

    unsafe fn with_nosave_interrupts<R, F: FnOnce(&mut Self::Target) -> R>(&self, f: F) -> R {
        f(&mut *self.get())
    }

    unsafe fn with_static_nosave_interrupts<R, F: FnOnce(&'static mut Self::Target) -> R>(
        &self,
        f: F,
    ) -> R {
        f(&mut *self.get())
    }

    fn get_for(&self, id: usize) -> *mut Self::Target
    where
        Self::Target: Sized,
    {
        unsafe {
            let value: *mut u8 = self.get().cast();
            let value = value.sub(per_cpu_base().as_u64() as usize);
            let value = value.add(per_cpu_base_for(id).as_u64() as usize);
            value.cast()
        }
    }

    fn clone(&self) -> Self::Target
    where
        Self::Target: Clone,
    {
        self.with(|v| v.clone())
    }

    fn copy(&self) -> Self::Target
    where
        Self::Target: Copy,
    {
        self.with(|v| *v)
    }
}

#[derive(Debug)]
pub struct CpuInfo {
    pub id: usize,
}

pub fn this_cpu_info() -> &'static CpuInfo {
    arch_this_cpu_info()
}
