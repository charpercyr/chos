use core::fmt::Debug;
use core::marker::PhantomData;

use bitflags::bitflags;

use crate::arch::mm::{PAddr, VAddr};
use crate::int::ceil_divu64;

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct FrameAlignError;

macro_rules! frame {
    ($name:ident : $addr:ty) => {
        #[repr(transparent)]
        #[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
        pub struct $name<S: FrameSize> {
            addr: $addr,
            size: PhantomData<S>,
        }

        crate::forward_fmt!(impl<S: FrameSize> Display, LowerHex, UpperHex for $name<S> => $addr : |this: &Self| this.addr);

        impl<S: FrameSize> $name<S> {
            pub const fn new(addr: $addr) -> Self {
                match Self::try_new(addr) {
                    Ok(addr) => addr,
                    Err(_) => panic!("Address is not frame aligned"),
                }
            }

            pub const fn try_new(addr: $addr) -> Result<Self, FrameAlignError> {
                if addr.as_u64() % S::PAGE_SIZE == 0 {
                    Ok(Self {
                        addr,
                        size: PhantomData,
                    })
                } else {
                    Err(FrameAlignError)
                }
            }

            pub fn new_align_up(addr: $addr) -> Self {
                Self {
                    addr: <$addr>::new(ceil_divu64(addr.as_u64(), S::PAGE_SIZE)),
                    size: PhantomData,
                }
            }

            pub fn new_align_down(addr: $addr) -> Self {
                Self {
                    addr: <$addr>::new(addr.as_u64() / S::PAGE_SIZE * S::PAGE_SIZE),
                    size: PhantomData,
                }
            }

            pub const unsafe fn new_unchecked(addr: $addr) -> Self {
                Self {
                    addr,
                    size: PhantomData,
                }
            }

            pub const fn addr(&self) -> $addr {
                self.addr
            }
        }

        paste::item! {
            pub struct [<$name Range>]<S: FrameSize> {
                start: $name<S>,
                end: $name<S>,
            }

            impl<S: FrameSize> [<$name Range>]<S> {
                pub const fn new(start: $name<S>, end: $name<S>) -> Self {
                    Self { start, end }
                }

                pub const fn start(&self) -> $name<S> {
                    self.start
                }

                pub const fn end(&self) -> $name<S> {
                    self.end
                }
            }

            impl<S: FrameSize> Iterator for [<$name Range>]<S> {
                type Item = $name<S>;
                fn next(&mut self) -> Option<Self::Item> {
                    if self.start.addr() < self.end.addr() {
                        let frame = self.start;
                        self.start = unsafe { $name::new_unchecked(self.start.addr() + S::PAGE_SIZE) };
                        Some(frame)
                    } else {
                        None
                    }
                }
            }
        }
    };
}
frame!(PFrame: PAddr);
frame!(VFrame: VAddr);

pub trait FrameSize: Clone + Copy + Debug {
    const PAGE_SIZE: u64;
    const DEBUG_STR: &'static str;
}

pub unsafe trait FrameAllocator<S: FrameSize> {
    type Error;
    unsafe fn alloc_frame(&mut self) -> Result<VFrame<S>, Self::Error>;
    unsafe fn dealloc_frame(&mut self, frame: VFrame<S>) -> Result<(), Self::Error>;
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
pub enum MapError<FE> {
    AlreadyMapped,
    FrameAllocError(FE),
}

impl<FE> MapError<FE> {
    pub const fn from_frame_alloc_error(fe: FE) -> Self {
        Self::FrameAllocError(fe)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum UnmapError<FE> {
    NotMapped,
    InvalidFrame,
    InvalidPage,
    FrameAllocError(FE),
}

pub trait Mapper<S: FrameSize> {
    type Flush: MapperFlush;
    type PGTFrameSize: FrameSize;
    unsafe fn map<A: FrameAllocator<Self::PGTFrameSize> + ?Sized>(
        &mut self,
        pframe: PFrame<S>,
        vframe: VFrame<S>,
        flags: MapFlags,
        alloc: &mut A,
    ) -> Result<Self::Flush, MapError<A::Error>>;

    unsafe fn unmap<A: FrameAllocator<Self::PGTFrameSize> + ?Sized>(
        &mut self,
        vframe: VFrame<S>,
        alloc: &mut A,
    ) -> Result<Self::Flush, UnmapError<A::Error>>;
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
    type PGTFrameSize = M::PGTFrameSize;
    unsafe fn map<A: FrameAllocator<Self::PGTFrameSize> + ?Sized>(
        &mut self,
        pframe: PFrame<S>,
        vframe: VFrame<S>,
        flags: MapFlags,
        alloc: &mut A,
    ) -> Result<Self::Flush, MapError<A::Error>> {
        crate::log::debug!("MAP {:016x} ({}) -> {:016x} [{:?}]", vframe, S::DEBUG_STR, pframe, flags);
        self.mapper.map(pframe, vframe, flags, alloc)
    }
    unsafe fn unmap<A: FrameAllocator<Self::PGTFrameSize> + ?Sized>(
        &mut self,
        frame: VFrame<S>,
        alloc: &mut A,
    ) -> Result<Self::Flush, UnmapError<A::Error>> {
        crate::log::debug!("UNMAP {:016x} ({})", frame, S::DEBUG_STR);
        self.mapper.unmap(frame, alloc)
    }
}
