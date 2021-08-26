#![no_std]

use core::fmt::Arguments;

use chos_x64::paging::{PAddr, VAddr};

mod macros;

pub mod phys {
    use super::PAddr;

    pub const KERNEL_DATA_BASE: PAddr = PAddr::new(0x0100_0000);
}

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
    pub early_log: fn(Arguments),
    pub mem_info: KernelMemInfo,
}

pub type KernelEntry = fn(&KernelBootInfo, u8) -> !;
