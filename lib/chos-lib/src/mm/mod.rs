use core::marker::PhantomData;

use bitflags::bitflags;

#[repr(transparent)]
#[derive(Debug, PartialEq, Eq, Hash, Clone, Copy)]
pub struct Frame<S: FrameSize> {
    virt: u64,
    size: PhantomData<S>,
}

pub struct Page<S: FrameSize> {
    phys: u64,
    size: PhantomData<S>,
}

pub trait FrameSize {
    const PAGE_SIZE: u64;
    const DEBUG_STR: &'static str;
}

pub unsafe trait FrameAlloc {
    unsafe fn alloc_frame<S: FrameSize>(&mut self) -> Frame<S>;
    unsafe fn dealloc_frame<S: FrameSize>(&mut self, frame: Frame<S>);
}

bitflags! {
    struct MapFlags: u8 {
        const WRITE =   0b0000_0001;
        const EXEC =    0b0000_0010;

        const GLOBAL =  0b0001_0000;
        const NOCACHE = 0b0010_0000;
        const USER =    0b0100_0000;
    }
}

pub trait Mapper<S: FrameSize> {
    unsafe fn map<A: FrameAlloc + ?Sized>(
        &mut self,
        page: Page<S>,
        frame: Frame<S>,
        flags: MapFlags,
        alloc: &mut A,
    );

    unsafe fn unmap<A: FrameAlloc + ?Sized>(&mut self, frame: Frame<S>, alloc: &mut A);
}
