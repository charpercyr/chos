mod addr;
use core::fmt::{self, Debug};
use core::marker::PhantomData;
use core::ops::{Deref, DerefMut};
use core::{cmp, hash};

pub use addr::*;
use bitflags::bitflags;

use crate::arch::mm::DefaultFrameSize;
use crate::elf::{Elf, ProgramEntryFlags, ProgramEntryType};
use crate::int::ceil_divu64;
use crate::log::{print, println};

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct FrameAlignError;

macro_rules! frame {
    ($name:ident : $addr:ty) => {
        #[repr(transparent)]
        pub struct $name<S: FrameSize = DefaultFrameSize> {
            addr: $addr,
            size: PhantomData<S>,
        }
        impl<S: FrameSize> Clone for $name<S> {
            fn clone(&self) -> Self {
                *self
            }
        }
        impl<S: FrameSize> Copy for $name<S> {}

        impl<S: FrameSize> PartialEq for $name<S> {
            fn eq(&self, rhs: &Self) -> bool {
                self.addr.eq(&rhs.addr)
            }
        }
        impl<S: FrameSize> Eq for $name<S> {}
        impl<S: FrameSize> PartialEq<$addr> for $name<S> {
            fn eq(&self, rhs: &$addr) -> bool {
                self.addr.eq(rhs)
            }
        }

        impl<S: FrameSize> PartialOrd for $name<S> {
            fn partial_cmp(&self, rhs: &Self) -> Option<cmp::Ordering> {
                self.addr.partial_cmp(&rhs.addr)
            }
        }
        impl<S: FrameSize> Ord for $name<S> {
            fn cmp(&self, rhs: &Self) -> cmp::Ordering {
                self.addr.cmp(&rhs.addr)
            }
        }
        impl<S: FrameSize> PartialOrd<$addr> for $name<S> {
            fn partial_cmp(&self, rhs: &$addr) -> Option<cmp::Ordering> {
                self.addr.partial_cmp(rhs)
            }
        }

        impl<S: FrameSize> hash::Hash for $name<S> {
            fn hash<H: hash::Hasher>(&self, h: &mut H) {
                self.addr.hash(h)
            }
        }

        crate::forward_fmt!(impl<S: FrameSize> Debug, Display, LowerHex, UpperHex for $name<S> => $addr : |this: &Self| this.addr);

        impl<S: FrameSize> $name<S> {
            pub const fn null() -> Self {
                Self {
                    addr: <$addr>::null(),
                    size: PhantomData,
                }
            }
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

            pub const fn new_align_up(addr: $addr) -> Self {
                Self {
                    addr: <$addr>::new(ceil_divu64(addr.as_u64(), S::PAGE_SIZE) * S::PAGE_SIZE),
                    size: PhantomData,
                }
            }

            pub const fn new_align_down(addr: $addr) -> Self {
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

            pub const fn add(self, count: u64) -> Self {
                unsafe { Self::new_unchecked(self.addr.add_u64(count * S::PAGE_SIZE)) }
            }

            pub const fn sub(self, count: u64) -> Self {
                unsafe { Self::new_unchecked(self.addr.sub_u64(count * S::PAGE_SIZE)) }
            }

            pub const fn page(self) -> u64 {
                self.addr.page()
            }
        }

        paste::item! {
            pub struct [<$name Range>]<S: FrameSize = DefaultFrameSize> {
                start: $name<S>,
                end: $name<S>,
            }

            impl<S: FrameSize> Clone for [<$name Range>]<S> {
                fn clone(&self) -> Self {
                    *self
                }
            }
            impl<S: FrameSize> Copy for [<$name Range>]<S> {}

            impl<S: FrameSize> PartialEq for [<$name Range>]<S> {
                fn eq(&self, rhs: &Self) -> bool {
                    self.start.eq(&rhs.start) && self.end.eq(&rhs.end)
                }
            }
            impl<S: FrameSize> Eq for [<$name Range>]<S> {}

            impl<S: FrameSize> [<$name Range>]<S> {
                pub const fn new(start: $name<S>, end: $name<S>) -> Self {
                    assert!(start.addr().as_u64() <= end.addr().as_u64());
                    Self { start, end }
                }

                pub const fn start(&self) -> $name<S> {
                    self.start
                }

                pub const fn end(&self) -> $name<S> {
                    self.end
                }

                pub const fn frame_count(&self) -> u64 {
                    (self.end.addr().as_u64() - self.start.addr().as_u64()) / S::PAGE_SIZE
                }

                pub fn contains(&self, rhs: &Self) -> bool {
                    self.start <= rhs.start && self.end >= rhs.end
                }

                pub fn intesects(&self, rhs: &Self) -> bool {
                    self.end >= rhs.start && rhs.end <= self.start
                }

                pub fn intersection(self, rhs: Self) -> Option<Self> {
                    if self.intesects(&rhs) {
                        Some(
                            Self::new(
                                <$name<S>>::max(self.start, rhs.start),
                                <$name<S>>::min(self.end, rhs.end)
                            )
                        )
                    } else {
                        None
                    }
                }

                pub fn contains_address(&self, rhs: $addr) -> bool {
                    self.start.addr() <= rhs && self.end.addr() > rhs
                }

                pub fn contains_frame(&self, rhs: $name<S>) -> bool {
                    self.contains_address(rhs.addr())
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

            impl<S: FrameSize> Debug for [<$name Range>]<S> {
                fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
                    f.debug_struct(stringify!([<$name Range>]))
                        .field("start", &self.start)
                        .field("end", &self.end)
                        .field("frame_size", &S::DEBUG_STR)
                        .finish()
                }
            }
        }
    };
}
frame!(PFrame: PAddr);
frame!(VFrame: VAddr);

impl<S: FrameSize> PFrame<S> {
    pub const fn identity(self) -> VFrame<S> {
        unsafe { VFrame::new_unchecked(self.addr.identity()) }
    }

    pub const fn offset(self, base: VFrame<S>) -> VFrame<S> {
        unsafe { VFrame::new_unchecked(base.addr.add_paddr(self.addr)) }
    }
}

impl<S: FrameSize> PFrameRange<S> {
    pub const fn identity(self) -> VFrameRange<S> {
        VFrameRange::new(self.start.identity(), self.end.identity())
    }
    pub const fn offset(self, base: VFrame<S>) -> VFrameRange<S> {
        VFrameRange::new(self.start.offset(base), self.end.offset(base))
    }
}

pub trait FrameSize: Clone + Copy + Debug {
    const PAGE_SHIFT: u8;
    const PAGE_SIZE: u64 = 1 << Self::PAGE_SHIFT;
    const PAGE_MASK: u64 = !(Self::PAGE_SIZE - 1);
    const DEBUG_STR: &'static str;
}

pub unsafe trait FrameAllocator<S: FrameSize> {
    type Error: fmt::Debug;
    unsafe fn alloc_frame(&mut self) -> Result<VFrame<S>, Self::Error>;
    unsafe fn dealloc_frame(&mut self, frame: VFrame<S>) -> Result<(), Self::Error>;
}

bitflags! {
    pub struct MapFlags: u32 {
        const WRITE =           0b0000_0000_0001;
        const EXEC =            0b0000_0000_0010;

        const GLOBAL =          0b0000_0001_0000;
        const NOCACHE =         0b0000_0010_0000;
        const USER =            0b0000_0100_0000;

        const IMM_ALL_CPUS =    0b0001_0000_0000;
    }
}

pub trait MapperFlush: Sized {
    const NONE: Self;

    fn flush(self);
    fn ignore(self) {
        drop(self)
    }

    fn combine(self, rhs: Self) -> Self;
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MapError<FE> {
    AlreadyMapped,
    FrameAllocError(FE),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MapElfError<FE> {
    AlreadyMapped,
    InvalidAlignment,
    FrameAllocError(FE),
}

impl<FE> From<MapError<FE>> for MapElfError<FE> {
    fn from(e: MapError<FE>) -> Self {
        match e {
            MapError::AlreadyMapped => Self::AlreadyMapped,
            MapError::FrameAllocError(fe) => Self::FrameAllocError(fe),
        }
    }
}

impl<FE> MapError<FE> {
    pub const fn from_frame_alloc_error(fe: FE) -> Self {
        Self::FrameAllocError(fe)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum UnmapError<FE> {
    NotMapped,
    InvalidSize,
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

pub trait RangeMapper<S: FrameSize>: Mapper<S> {
    unsafe fn map_range<A: FrameAllocator<Self::PGTFrameSize> + ?Sized>(
        &mut self,
        prange: PFrameRange<S>,
        vbase: VFrame<S>,
        flags: MapFlags,
        alloc: &mut A,
    ) -> Result<Self::Flush, MapError<A::Error>>;

    unsafe fn unmap_range<A: FrameAllocator<Self::PGTFrameSize> + ?Sized>(
        &mut self,
        vrange: VFrameRange<S>,
        alloc: &mut A,
    ) -> Result<Self::Flush, UnmapError<A::Error>>;

    unsafe fn map_elf_load_sections<A: FrameAllocator<Self::PGTFrameSize> + ?Sized>(
        &mut self,
        elf: &Elf,
        pbase: PFrame<S>,
        vbase: VFrame<S>,
        base_flags: MapFlags,
        alloc: &mut A,
    ) -> Result<Self::Flush, MapElfError<A::Error>> {
        let load_sections = elf
            .program()
            .iter()
            .filter(|e| e.typ() == ProgramEntryType::Load);
        for e in load_sections.clone() {
            if e.align() != S::PAGE_SIZE {
                return Err(MapElfError::InvalidAlignment);
            }
        }
        let mut total_flush = Self::Flush::NONE;
        for e in load_sections {
            let mut flags = base_flags;
            if e.flags().contains(ProgramEntryFlags::EXEC) {
                flags |= MapFlags::EXEC;
            }
            if e.flags().contains(ProgramEntryFlags::WRITE) {
                flags |= MapFlags::WRITE;
            }
            let flush = self.map_range(
                PFrameRange::new(
                    PFrame::new_align_down(pbase.addr() + PAddr::new(e.paddr())),
                    PFrame::new_align_up(pbase.addr() + PAddr::new(e.paddr()) + e.mem_size()),
                ),
                VFrame::new_align_down(vbase.addr() + e.paddr()),
                flags,
                alloc,
            )?;
            total_flush = total_flush.combine(flush);
        }
        Ok(total_flush)
    }
}

pub trait PAddrResolver {
    fn paddr_of(&self, vaddr: VAddr) -> Option<PAddr>;
}

pub struct LoggingMapper<M> {
    inner: M,
}
impl<M> LoggingMapper<M> {
    pub const fn new(inner: M) -> Self {
        Self { inner }
    }

    pub fn inner(&self) -> &M {
        &self.inner
    }

    pub fn inner_mut(&mut self) -> &mut M {
        &mut self.inner
    }
}
impl<M> Deref for LoggingMapper<M> {
    type Target = M;
    fn deref(&self) -> &M {
        &self.inner
    }
}
impl<M> DerefMut for LoggingMapper<M> {
    fn deref_mut(&mut self) -> &mut M {
        &mut self.inner
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
        crate::log::debug!(
            "MAP {:016x} ({}) -> {:016x} [{:?}]",
            vframe,
            S::DEBUG_STR,
            pframe,
            flags
        );
        self.inner.map(pframe, vframe, flags, alloc)
    }
    unsafe fn unmap<A: FrameAllocator<Self::PGTFrameSize> + ?Sized>(
        &mut self,
        frame: VFrame<S>,
        alloc: &mut A,
    ) -> Result<Self::Flush, UnmapError<A::Error>> {
        crate::log::debug!("UNMAP {:016x} ({})", frame, S::DEBUG_STR);
        self.inner.unmap(frame, alloc)
    }
}

impl<S: FrameSize, M: RangeMapper<S>> RangeMapper<S> for LoggingMapper<M> {
    unsafe fn map_range<A: FrameAllocator<Self::PGTFrameSize> + ?Sized>(
        &mut self,
        prange: PFrameRange<S>,
        vbase: VFrame<S>,
        flags: MapFlags,
        alloc: &mut A,
    ) -> Result<Self::Flush, MapError<A::Error>> {
        crate::log::debug!(
            "MAP {:016x}-{:016x} ({}) -> {:016x}-{:016x} [{:?}]",
            vbase,
            vbase.add(prange.frame_count()),
            S::DEBUG_STR,
            prange.start(),
            prange.end(),
            flags,
        );
        self.inner.map_range(prange, vbase, flags, alloc)
    }
    unsafe fn unmap_range<A: FrameAllocator<Self::PGTFrameSize> + ?Sized>(
        &mut self,
        vrange: VFrameRange<S>,
        alloc: &mut A,
    ) -> Result<Self::Flush, UnmapError<A::Error>> {
        crate::log::debug!(
            "UNMAP {:016x}-{:016x} ({})",
            vrange.start(),
            vrange.end(),
            S::DEBUG_STR
        );
        self.inner.unmap_range(vrange, alloc)
    }
}

impl<M: PAddrResolver> PAddrResolver for LoggingMapper<M> {
    fn paddr_of(&self, vaddr: VAddr) -> Option<PAddr> {
        self.inner.paddr_of(vaddr)
    }
}

pub trait MapVisitor<'a> {
    type Entry;
    type Iterator: Iterator<Item = Self::Entry>;
    fn root(&'a self) -> Self::Iterator;
    fn children(&'a self, e: Self::Entry) -> Option<Self::Iterator>;
}

pub fn print_mapping<'a>(m: &'a impl MapVisitor<'a, Entry: fmt::Debug>) {
    fn do_print_mapping<'a, M: MapVisitor<'a, Entry: fmt::Debug>>(
        m: &'a M,
        it: M::Iterator,
        lvl: usize,
    ) {
        for e in it {
            for _ in 0..lvl {
                print!("|  ");
            }
            println!("{:x?}", e);
            if let Some(it) = m.children(e) {
                do_print_mapping(m, it, lvl + 1);
            }
        }
    }
    do_print_mapping(m, m.root(), 0)
}

impl<'a, M: MapVisitor<'a>> MapVisitor<'a> for LoggingMapper<M> {
    type Entry = M::Entry;
    type Iterator = M::Iterator;
    fn root(&'a self) -> Self::Iterator {
        self.inner.root()
    }
    fn children(&'a self, e: Self::Entry) -> Option<Self::Iterator> {
        self.inner.children(e)
    }
}
