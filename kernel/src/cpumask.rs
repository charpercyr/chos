use chos_lib::cpumask::Cpumask;
use chos_lib::sync::SpinOnceCell;

use crate::mm::this_cpu_info;

static ALL_CPUS: SpinOnceCell<Cpumask> = SpinOnceCell::new();
pub fn init_cpumask(core_count: usize) {
    let mask = (1 << core_count) - 1;
    SpinOnceCell::force_set(&ALL_CPUS, Cpumask::from_raw(mask)).unwrap();
}

pub fn all() -> Cpumask {
    *ALL_CPUS.try_get().expect("Cpumask not initialized")
}

pub fn all_but_self() -> Cpumask {
    all() - Cpumask::cpu(this_cpu_info().id as u8)
}
