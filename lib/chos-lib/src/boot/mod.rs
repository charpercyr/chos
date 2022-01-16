use core::ptr::NonNull;

use crate::arch::boot::ArchKernelBootInfo;
use crate::arch::x64::mm::PAddr;
use crate::log::LogHandler;

#[derive(Copy, Clone, Debug)]
pub struct KernelMemEntry {
    pub phys: PAddr,
    pub size: usize,
}

#[derive(Copy, Clone, Debug)]
pub struct KernelMemInfo {
    pub code: KernelMemEntry,
    pub total_size: u64,
}

#[derive(Copy, Clone)]
pub struct KernelBootInfo {
    pub core_count: usize,
    pub elf: NonNull<[u8]>,
    pub initrd: Option<NonNull<[u8]>>,
    pub early_log: &'static dyn LogHandler,
    pub mem_info: KernelMemInfo,
    pub arch: ArchKernelBootInfo,
}

pub type KernelEntry = fn(&KernelBootInfo, usize) -> !;

#[macro_export]
macro_rules! check_kernel_entry {
    ($entry:expr) => {
        const _: $crate::boot::KernelEntry = $entry;
    };
}
