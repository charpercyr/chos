pub mod phys {
    use chos_lib::arch::mm::FrameSize4K;
    use chos_lib::mm::PFrame;

    // Extra space for the size of the debug ELF (extra 40 MB)
    #[cfg(debug_assertions)]
    pub const KERNEL_DATA_BASE: PFrame<FrameSize4K> = unsafe {
        use chos_lib::mm::PAddr;
        PFrame::new_unchecked(PAddr::new(0x0400_0000))
    };
    #[cfg(not(debug_assertions))]
    pub const KERNEL_DATA_BASE: PAddr = PAddr::new(0x0100_0000);
}

pub mod virt {
    use chos_lib::arch::mm::FrameSize4K;
    use chos_lib::mm::{FrameSize, VAddr, VFrame};

    pub const KERNEL_BASE: VFrame<FrameSize4K> =
        unsafe { VFrame::new_unchecked(VAddr::new_unchecked(0xffff_8000_0000_0000)) };
    pub const MEMORY_ZONE_FRAMES: u64 = 0x0080_0000_0000 / FrameSize4K::PAGE_SIZE;

    pub const PHYSICAL_MAP_BASE: VFrame<FrameSize4K> = KERNEL_BASE;
    pub const STATIC_BASE: VFrame<FrameSize4K> = KERNEL_BASE.add(1 * MEMORY_ZONE_FRAMES);
    pub const HEAP_BASE: VFrame<FrameSize4K> = KERNEL_BASE.add(2 * MEMORY_ZONE_FRAMES);
    pub const DEVICE_BASE: VFrame<FrameSize4K> = KERNEL_BASE.add(3 * MEMORY_ZONE_FRAMES);
    pub const PER_CPU_BASE: VFrame<FrameSize4K> = KERNEL_BASE.add(4 * MEMORY_ZONE_FRAMES);
    pub const STACK_BASE: VFrame<FrameSize4K> = KERNEL_BASE.add(5 * MEMORY_ZONE_FRAMES);
}

pub mod stack {
    use chos_lib::arch::mm::PAGE_SIZE;

    #[cfg(debug_assertions)]
    pub const KERNEL_STACK_PAGE_ORDER: u8 = 5;
    #[cfg(not(debug_assertions))]
    pub const KERNEL_STACK_PAGE_ORDER: u8 = 4;

    pub const KERNEL_STACK_SIZE: usize = PAGE_SIZE << KERNEL_STACK_PAGE_ORDER;
}
