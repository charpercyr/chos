#![no_std]

use core::fmt::Arguments;

mod macros;

pub mod phys {
    pub const KERNEL_DATA_BASE: u64 = 0x0100_0000;
}

pub mod virt {
    pub const KERNEL_CODE_BASE: u64 = 0xffff_8000_4000_0000;
    pub const KERNEL_PT_BASE: u64 =   0xffff_8000_8000_0000;
}

#[derive(Copy, Clone)]
pub struct KernelBootInfo {
    pub multiboot_header: usize,
    pub elf: usize,
    pub early_log: fn(Arguments),
}

pub type KernelEntry = fn(&KernelBootInfo, u8) -> !;
