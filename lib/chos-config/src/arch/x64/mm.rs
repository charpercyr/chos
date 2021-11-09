
pub mod phys { 
    use chos_lib::arch::mm::PAddr;

    pub const KERNEL_DATA_BASE: PAddr = PAddr::new(0x0100_0000);
}

pub mod virt {
    use chos_lib::arch::mm::VAddr;
    
    pub const KERNEL_BASE: VAddr = unsafe { VAddr::new_unchecked(0xffff_8000_0000_0000) };
    pub const MEMORY_ZONE_SIZE: u64 = 0x0080_0000_0000;
    
    pub const PHYSICAL_MAP_BASE: VAddr = KERNEL_BASE.add_u64(0 * MEMORY_ZONE_SIZE);
    pub const PAGING_BASE: VAddr = KERNEL_BASE.add_u64(1 * MEMORY_ZONE_SIZE);
    pub const DEVICE_BASE: VAddr = KERNEL_BASE.add_u64(2 * MEMORY_ZONE_SIZE);
    pub const STATIC_BASE: VAddr = KERNEL_BASE.add_u64(3 * MEMORY_ZONE_SIZE);
    pub const HEAP_BASE: VAddr = KERNEL_BASE.add_u64(4 * MEMORY_ZONE_SIZE);
    pub const PERCPU_STATIC_BASE: VAddr = KERNEL_BASE.add_u64(5 * MEMORY_ZONE_SIZE);
    pub const PERCPU_HEAP_BASE: VAddr = KERNEL_BASE.add_u64(6 * MEMORY_ZONE_SIZE);
    pub const STACK_BASE: VAddr = KERNEL_BASE.add_u64(7 * MEMORY_ZONE_SIZE);
}
