pub mod phys {
    use chos_lib::arch::mm::PAddr;

    // Extra space for the size of the debug ELF (extra 40 MB)
    #[cfg(debug_assertions)]
    pub const KERNEL_DATA_BASE: PAddr = PAddr::new(0x0400_0000);
    #[cfg(not(debug_assertions))]
    pub const KERNEL_DATA_BASE: PAddr = PAddr::new(0x0100_0000);
}

pub mod virt {
    use chos_lib::arch::mm::VAddr;

    pub const KERNEL_BASE: VAddr = unsafe { VAddr::new_unchecked(0xffff_8000_0000_0000) };
    pub const MEMORY_ZONE_SIZE: u64 = 0x0080_0000_0000;

    pub const PHYSICAL_MAP_BASE: VAddr = KERNEL_BASE;
    pub const STATIC_BASE: VAddr = KERNEL_BASE.add_u64(1 * MEMORY_ZONE_SIZE);
    pub const HEAP_BASE: VAddr = KERNEL_BASE.add_u64(2 * MEMORY_ZONE_SIZE);
    pub const DEVICE_BASE: VAddr = KERNEL_BASE.add_u64(3 * MEMORY_ZONE_SIZE);
    pub const PER_CPU_BASE: VAddr = KERNEL_BASE.add_u64(4 * MEMORY_ZONE_SIZE);
    pub const STACK_BASE: VAddr = KERNEL_BASE.add_u64(5 * MEMORY_ZONE_SIZE);
}

pub mod stack {
    use chos_lib::arch::mm::PAGE_SIZE;

    #[cfg(debug_assertions)]
    pub const KERNEL_STACK_PAGE_ORDER: u8 = 5;
    #[cfg(not(debug_assertions))]
    pub const KERNEL_STACK_PAGE_ORDER: u8 = 4;

    pub const KERNEL_STACK_SIZE: usize = PAGE_SIZE << KERNEL_STACK_PAGE_ORDER;
}
