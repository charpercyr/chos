use core::mem::MaybeUninit;

pub use chos_lib::cpumask::Cpumask;
use chos_lib::sync::SpinOnceCell;

use crate::mm::this_cpu_info;

static mut CPU_COUNT: MaybeUninit<usize> = MaybeUninit::uninit();
static ALL_CPUS: SpinOnceCell<Cpumask> = SpinOnceCell::new();
pub fn init_cpumask(core_count: usize) {
    unsafe { CPU_COUNT = MaybeUninit::new(core_count) };
    let mask = (1 << core_count) - 1;
    SpinOnceCell::force_set(&ALL_CPUS, Cpumask::from_raw(mask)).unwrap();
}

#[inline]
pub fn cpu_count() -> usize {
    unsafe { CPU_COUNT.assume_init() }
}

pub fn all() -> Cpumask {
    *ALL_CPUS.try_get().expect("Cpumask not initialized")
}

pub fn this_cpu() -> Cpumask {
    Cpumask::for_cpu(this_cpu_info().id as u8)
}

pub fn all_but_this_cpu() -> Cpumask {
    all() - this_cpu()
}
