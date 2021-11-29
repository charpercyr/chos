use core::alloc::AllocError;

use chos_config::arch::mm::virt;
use chos_lib::arch::mm::{FrameSize4K, PAddr};
use chos_lib::init::ConstInit;
use chos_lib::mm::{FrameAllocator, VFrame};

use crate::mm::phys::{raw_alloc, AllocFlags};

struct MMFrameAllocator;

unsafe impl FrameAllocator<FrameSize4K> for MMFrameAllocator {
    type Error = AllocError;
    unsafe fn alloc_frame(&mut self) -> Result<VFrame<FrameSize4K>, Self::Error> {
        raw_alloc::alloc_pages(0, AllocFlags::empty())
            .map(|p| VFrame::new_unchecked(p + virt::PHYSICAL_MAP_BASE))
    }
    unsafe fn dealloc_frame(&mut self, frame: VFrame<FrameSize4K>) -> Result<(), Self::Error> {
        raw_alloc::dealloc_pages(
            PAddr::new((frame.addr() - virt::PHYSICAL_MAP_BASE).as_u64()),
            0,
        );
        Ok(())
    }
}

pub struct ArchVMMap {}

impl ConstInit for ArchVMMap {
    const INIT: Self = Self {};
}

pub struct ArchVMArea {}
