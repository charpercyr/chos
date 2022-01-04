mod global;
mod per_cpu;
pub mod phys;
pub mod slab;
pub mod virt;

use chos_lib::boot::KernelBootInfo;
pub use per_cpu::*;

use crate::arch::mm::arch_init_early_memory;

pub unsafe fn init_early_memory(info: &KernelBootInfo) {
    arch_init_early_memory(info)
}
