#![no_std]

pub mod phys {
    pub const KERNEL_DATA_BASE: usize = 0x0100_0000;
}

pub mod virt {
    pub const KERNEL_CODE_BASE: usize = 0xffff_8000_4000_0000;
    pub const KERNEL_PT_BASE: usize =   0xffff_8000_8000_0000;
}

pub enum BootMemoryType {
    Boot,
    Stack,
    Kernel,
    PageTable,
}

pub struct BootMemoryEntry {
    pub phys_addr: usize,
    pub virt_addr: usize,
    pub len: usize,
    pub typ: BootMemoryType,
}

#[repr(C)]
pub struct BootMemoryMap {
    pub entries: *const BootMemoryEntry,
    pub len: usize,
}

#[repr(C)]
pub struct KernelBootInfo {
    pub entry: usize,
    pub multiboot_header: usize,
    pub elf: usize,
    pub memory_map: BootMemoryMap,
}