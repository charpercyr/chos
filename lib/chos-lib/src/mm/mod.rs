use core::marker::PhantomData;

#[repr(transparent)]
#[derive(Debug, PartialEq, Eq, Hash, Clone, Copy)]
pub struct Frame<S: FrameSize> {
    phys: u64,
    size: PhantomData<S>,
}

impl<S: FrameSize> Frame<S> {}

pub trait FrameSize {
    const PAGE_SIZE: u64;
}

pub unsafe trait FrameAlloc {
    unsafe fn alloc_frame<S: FrameSize>(&mut self) -> Frame<S>;
    unsafe fn dealloc_frame<S: FrameSize>(&mut self, frame: Frame<S>);
}

pub unsafe trait Mapper<S: FrameSize> {
    unsafe fn map(
        &mut self,
        paddr: u64,
        vaddr: u64,
        alloc: &mut impl FrameAlloc,
    ) -> Result<Frame<S>, ()>;

    unsafe fn unmap(&mut self, frame: Frame<S>, alloc: &mut impl FrameAlloc);
}
