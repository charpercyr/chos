use core::marker::PhantomData;

use bitflags::bitflags;

use crate::arch::mm::{PAddr, VAddr};
use crate::int::ceil_divu64;

#[repr(transparent)]
#[derive(Debug, PartialEq, Eq, Hash, Clone, Copy)]
pub struct Frame<S: FrameSize> {
    addr: VAddr,
    size: PhantomData<S>,
}

crate::forward_fmt!(
    impl<S: FrameSize> LowerHex, UpperHex for Frame<S> => VAddr : |this: &Self| this.addr
);

#[derive(Clone, Copy, Debug)]
pub struct FrameAlignError;

impl<S: FrameSize> Frame<S> {
    pub const fn try_new(addr: VAddr) -> Result<Self, FrameAlignError> {
        if addr.as_u64() % S::PAGE_SIZE != 0 {
            Err(FrameAlignError)
        } else {
            Ok(Self {
                addr,
                size: PhantomData,
            })
        }
    }

    pub const fn new_align_up(addr: VAddr) -> Self {
        Self {
            addr: VAddr::make_canonical(ceil_divu64(addr.as_u64(), S::PAGE_SIZE)),
            size: PhantomData,
        }
    }

    pub fn new_align_down(addr: VAddr) -> Self {
        Self {
            addr: VAddr::make_canonical(addr.as_u64() / S::PAGE_SIZE * S::PAGE_SIZE),
            size: PhantomData,
        }
    }

    pub const unsafe fn new_unchecked(addr: VAddr) -> Self {
        Self {
            addr,
            size: PhantomData,
        }
    }

    pub const fn addr(&self) -> VAddr {
        self.addr
    }
}

#[derive(Debug, Clone, Copy)]
pub struct FrameRange<S: FrameSize> {
    start: Frame<S>,
    end: Frame<S>,
}

impl<S: FrameSize> FrameRange<S> {
    pub const fn new(start: Frame<S>, end: Frame<S>) -> Self {
        Self { start, end }
    }

    pub const fn from_start_count(start: Frame<S>, count: u64) -> Self {
        Self {
            start,
            end: unsafe { Frame::new_unchecked(start.addr().add_canonical(count * S::PAGE_SIZE)) },
        }
    }
}

impl<S: FrameSize> Iterator for FrameRange<S> {
    type Item = Frame<S>;
    fn next(&mut self) -> Option<Self::Item> {
        if self.start.addr() < self.end.addr() {
            let frame = self.start;
            self.start =
                unsafe { Frame::new_unchecked(self.start.addr().add_canonical(S::PAGE_SIZE)) };
            Some(frame)
        } else {
            None
        }
    }
}

pub struct Page<S: FrameSize> {
    addr: PAddr,
    size: PhantomData<S>,
}

crate::forward_fmt!(
    impl<S: FrameSize> LowerHex, UpperHex for Page<S> => PAddr : |this: &Self| this.addr
);

#[derive(Clone, Copy, Debug)]
pub struct PageAlignError;

impl<S: FrameSize> Page<S> {
    pub const fn new(addr: PAddr) -> Result<Self, PageAlignError> {
        if addr.as_u64() % S::PAGE_SIZE == 0 {
            Ok(Self {
                addr,
                size: PhantomData,
            })
        } else {
            Err(PageAlignError)
        }
    }

    pub const fn new_align_up(addr: PAddr) -> Self {
        Self {
            addr: PAddr::new(ceil_divu64(addr.as_u64(), S::PAGE_SIZE)),
            size: PhantomData,
        }
    }

    pub const fn new_align_down(addr: PAddr) -> Self {
        Self {
            addr: PAddr::new(addr.as_u64() / S::PAGE_SIZE * S::PAGE_SIZE),
            size: PhantomData,
        }
    }

    pub const unsafe fn new_unchecked(addr: PAddr) -> Self {
        Self {
            addr,
            size: PhantomData,
        }
    }

    pub const fn addr(&self) -> PAddr {
        self.addr
    }
}

pub trait FrameSize: Clone + Copy {
    const PAGE_SIZE: u64;
    const DEBUG_STR: &'static str;
}

pub unsafe trait FrameAllocator {
    unsafe fn alloc_frame<S: FrameSize>(&mut self) -> Frame<S>;
    unsafe fn dealloc_frame<S: FrameSize>(&mut self, frame: Frame<S>);
}

bitflags! {
    pub struct MapFlags: u8 {
        const WRITE =   0b0000_0001;
        const EXEC =    0b0000_0010;

        const GLOBAL =  0b0001_0000;
        const NOCACHE = 0b0010_0000;
        const USER =    0b0100_0000;
    }
}

pub trait MapperFlush: Sized {
    fn flush(self);
    fn ignore(self) {
        drop(self)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MapError {
    AlreadyMapped,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum UnmapError {
    NotMapped,
    InvalidFrame,
    InvalidPage,
}

pub trait Mapper<S: FrameSize> {
    type Flush: MapperFlush;
    unsafe fn map<A: FrameAllocator + ?Sized>(
        &mut self,
        page: Page<S>,
        frame: Frame<S>,
        flags: MapFlags,
        alloc: &mut A,
    ) -> Result<Self::Flush, MapError>;

    unsafe fn unmap<A: FrameAllocator + ?Sized>(
        &mut self,
        frame: Frame<S>,
        alloc: &mut A,
    ) -> Result<Self::Flush, UnmapError>;

    // unsafe fn map_range<A: FrameAlloc + ?Sized>(
    //     &mut self,
    //     page: Page<S>,
    //     range: FrameRange<S>,
    //     flags: MapFlags,
    //     alloc: &mut A,
    // ) -> Result<Self::Flush, MapError>;
}

pub struct LoggingMapper<M> {
    mapper: M,
}
impl<M> LoggingMapper<M> {
    pub const fn new(mapper: M) -> Self {
        Self { mapper }
    }

    pub fn inner(&self) -> &M {
        &self.mapper
    }

    pub fn inner_mut(&mut self) -> &mut M {
        &mut self.mapper
    }
}
impl<S: FrameSize, M: Mapper<S>> Mapper<S> for LoggingMapper<M> {
    type Flush = M::Flush;
    unsafe fn map<A: FrameAllocator + ?Sized>(
        &mut self,
        page: Page<S>,
        frame: Frame<S>,
        flags: MapFlags,
        alloc: &mut A,
    ) -> Result<Self::Flush, MapError> {
        crate::log::debug!("MAP {:016x} -> {:016x} [{:?}]", page, frame, flags);
        self.mapper.map(page, frame, flags, alloc)
    }
    unsafe fn unmap<A: FrameAllocator + ?Sized>(
        &mut self,
        frame: Frame<S>,
        alloc: &mut A,
    ) -> Result<Self::Flush, UnmapError> {
        crate::log::debug!("UNMAP {:016x}", frame);
        self.mapper.unmap(frame, alloc)
    }
}
