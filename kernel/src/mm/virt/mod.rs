use chos_config::arch::mm::virt::PAGING_BASE;
use chos_lib::{arch::mm::{OffsetMapper, PageTable}, mm::FrameAllocator};

pub struct MapperAlloc;

unsafe impl FrameAllocator for MapperAlloc {
    type Error = !;
    unsafe fn alloc_frame<S: chos_lib::mm::FrameSize>(&mut self) -> Result<chos_lib::mm::Frame<S>, Self::Error> {
        todo!()
    }
    unsafe fn dealloc_frame<S: chos_lib::mm::FrameSize>(&mut self, frame: chos_lib::mm::Frame<S>) -> Result<(), Self::Error> {
        todo!()
    }
}

pub unsafe fn create_mapper(pgt: &mut PageTable) -> OffsetMapper<'_> {
    OffsetMapper::new(pgt, PAGING_BASE)
}
