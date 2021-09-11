
use crate::arch::x64::mm::{PAddr, VAddr};

mod macros;

#[derive(Copy, Clone, Debug)]
pub struct KernelMemEntry {
    pub virt: VAddr,
    pub phys: PAddr,
    pub size: usize,
}

#[derive(Copy, Clone, Debug)]
pub struct KernelMemInfo {
    pub code: KernelMemEntry,
    pub pt: KernelMemEntry,
}

#[derive(Copy, Clone)]
pub struct KernelBootInfo {
    pub multiboot_header: usize,
    pub elf: usize,
    pub early_log: crate::log::LogHandler,
    pub mem_info: KernelMemInfo,
}

pub type KernelEntry = fn(&KernelBootInfo, u8) -> !;

#[macro_export]
macro_rules! check_kernel_entry {
    ($entry:expr) => {
        const _: $crate::boot::KernelEntry = $entry;
    };
}
