use super::{FrameSize1G, FrameSize2M, FrameSize4K, PAddr, PageEntry, PageTable, VAddr};
use crate::mm::*;

#[must_use = "Must flush or ignore"]
pub enum Flush<S: FrameSize> {
    All,
    PageRange(FrameRange<S>),
    None,
}

impl<S: FrameSize> MapperFlush for Flush<S> {
    fn flush(self) {
        match self {
            Self::All => unsafe {
                asm! {
                    "mov %cr3, {tmp}",
                    "mov {tmp}, %cr3",
                    tmp = out(reg) _,
                    options(att_syntax, nomem, nostack)
                }
            },
            Self::PageRange(range) => unsafe {
                for frame in range {
                    asm! {
                        "invlpg ({addr})",
                        addr = in(reg) frame.addr().as_u64(),
                        options(att_syntax, nostack),
                    }
                }
            },
            Self::None => (),
        }
    }
}

pub struct OffsetMapper<'a> {
    pub p4: &'a mut PageTable,
    base: VAddr,
}

impl<'a> OffsetMapper<'a> {
    pub unsafe fn new(p4: &'a mut PageTable, base: VAddr) -> Self {
        Self { p4, base }
    }
    pub unsafe fn identity(p4: &'a mut PageTable) -> Self {
        Self {
            p4,
            base: VAddr::null(),
        }
    }
}

impl Mapper<FrameSize4K> for OffsetMapper<'_> {
    type Flush = Flush<FrameSize4K>;
    unsafe fn map<A: FrameAllocator + ?Sized>(
        &mut self,
        page: Page<FrameSize4K>,
        frame: Frame<FrameSize4K>,
        flags: MapFlags,
        alloc: &mut A,
    ) -> Result<Self::Flush, MapError> {
        let (p4i, p3i, p2i, p1i, _) = frame.addr().split();
        let (p3, p3a) = get_page_or_alloc::<FrameSize4K, _>(self.p4, self.base, flags, alloc, p4i);
        let (p2, p2a) = get_page_or_alloc::<FrameSize4K, _>(p3, self.base, flags, alloc, p3i);
        let (p1, p1a) = get_page_or_alloc::<FrameSize4K, _>(p2, self.base, flags, alloc, p2i);
        let entry = p1[p1i];
        if entry.present() {
            if p1a {
                alloc.dealloc_frame::<FrameSize4K>(Frame::new_unchecked(VAddr::new_unchecked(p1 as *mut PageTable as u64)));
                p2[p2i] = PageEntry::zero();
            }
            if p2a {
                alloc.dealloc_frame::<FrameSize4K>(Frame::new_unchecked(VAddr::new_unchecked(p2 as *mut PageTable as u64)));
                p3[p3i] = PageEntry::zero();
            }
            if p3a {
                alloc.dealloc_frame::<FrameSize4K>(Frame::new_unchecked(VAddr::new_unchecked(p3 as *mut PageTable as u64)));
                self.p4[p4i] = PageEntry::zero();
            }
            return Err(MapError::AlreadyMapped);
        }
        p1[p1i] = create_page_entry(page.addr(), flags);
        Ok(Flush::All)
    }

    unsafe fn unmap<A: FrameAllocator + ?Sized>(
        &mut self,
        _frame: Frame<FrameSize4K>,
        _alloc: &mut A,
    ) -> Result<Self::Flush, UnmapError> {
        Err(UnmapError::NotMapped)
    }
}

impl Mapper<FrameSize2M> for OffsetMapper<'_> {
    type Flush = Flush<FrameSize2M>;
    unsafe fn map<A: FrameAllocator + ?Sized>(
        &mut self,
        page: Page<FrameSize2M>,
        frame: Frame<FrameSize2M>,
        flags: MapFlags,
        alloc: &mut A,
    ) -> Result<Self::Flush, MapError> {
        let (p4i, p3i, p2i, _, _) = frame.addr().split();
        let (p3, p3a) = get_page_or_alloc::<FrameSize2M, _>(self.p4, self.base, flags, alloc, p4i);
        let (p2, p2a) = get_page_or_alloc::<FrameSize2M, _>(p3, self.base, flags, alloc, p3i);
        let entry = p2[p2i];
        if entry.present() {
            if p2a {
                alloc.dealloc_frame::<FrameSize2M>(Frame::new_unchecked(VAddr::new_unchecked(p2 as *mut PageTable as u64)));
                p3[p3i] = PageEntry::zero();
            }
            if p3a {
                alloc.dealloc_frame::<FrameSize2M>(Frame::new_unchecked(VAddr::new_unchecked(p3 as *mut PageTable as u64)));
                self.p4[p4i] = PageEntry::zero();
            }
            return Err(MapError::AlreadyMapped);
        }
        let mut entry = create_page_entry(page.addr(), flags);
        entry.set_huge_page(true);
        p2[p2i] = entry;
        Ok(Flush::All)
    }

    unsafe fn unmap<A: FrameAllocator + ?Sized>(
        &mut self,
        _frame: Frame<FrameSize2M>,
        _alloc: &mut A,
    ) -> Result<Self::Flush, UnmapError> {
        Err(UnmapError::NotMapped)
    }
}

impl Mapper<FrameSize1G> for OffsetMapper<'_> {
    type Flush = Flush<FrameSize1G>;
    unsafe fn map<A: FrameAllocator + ?Sized>(
        &mut self,
        page: Page<FrameSize1G>,
        frame: Frame<FrameSize1G>,
        flags: MapFlags,
        alloc: &mut A,
    ) -> Result<Self::Flush, MapError> {
        let (p4i, p3i, _, _, _) = frame.addr().split();
        let (p3, p3a) = get_page_or_alloc::<FrameSize1G, _>(self.p4, self.base, flags, alloc, p4i);
        let entry = p3[p3i];
        if entry.present() {
            if p3a {
                alloc.dealloc_frame::<FrameSize1G>(Frame::new_unchecked(VAddr::new_unchecked(p3 as *mut PageTable as u64)));
                self.p4[p4i] = PageEntry::zero();
            }
            return Err(MapError::AlreadyMapped);
        }
        let mut entry = create_page_entry(page.addr(), flags);
        entry.set_huge_page(true);
        p3[p3i] = entry;
        Ok(Flush::All)
    }

    unsafe fn unmap<A: FrameAllocator + ?Sized>(
        &mut self,
        _frame: Frame<FrameSize1G>,
        _alloc: &mut A,
    ) -> Result<Self::Flush, UnmapError> {
        Err(UnmapError::NotMapped)
    }
}

unsafe fn resolve_page_vaddr(base: VAddr, addr: PAddr) -> VAddr {
    VAddr::new_unchecked(addr.as_u64() + base.as_u64())
}

unsafe fn resolve_page_paddr(base: VAddr, addr: VAddr) -> PAddr {
    PAddr::new(addr.as_u64() - base.as_u64())
}

unsafe fn get_page_or_alloc<'p, S: FrameSize, A: FrameAllocator + ?Sized>(
    table: &'p mut PageTable,
    base: VAddr,
    flags: MapFlags,
    alloc: &mut A,
    i: u16,
) -> (&'p mut PageTable, bool) {
    let mut entry = table[i];
    let mut allocated = false;
    if !entry.present() {
        let frame = alloc.alloc_frame::<S>();
        entry = create_page_entry(resolve_page_paddr(base, frame.addr()), flags);
        allocated = true;
    } else {
        update_page_entry(&mut entry, flags);
    }
    table[i] = entry;
    let addr = resolve_page_vaddr(base, entry.phys_addr());
    (&mut *(addr.as_u64() as *mut PageTable), allocated)
}

fn create_page_entry(paddr: PAddr, flags: MapFlags) -> PageEntry {
    let mut entry = PageEntry::zero();
    entry.set_phys_addr(paddr);
    if !flags.contains(MapFlags::EXEC) {
        entry.set_no_execute(true);
    }
    if flags.contains(MapFlags::WRITE) {
        entry.set_writable(true);
    }
    if flags.contains(MapFlags::GLOBAL) {
        entry.set_global(true);
    }
    if flags.contains(MapFlags::USER) {
        entry.set_user(true);
    }
    if flags.contains(MapFlags::NOCACHE) {
        entry.set_no_cache(true);
        entry.set_write_through(true);
    }
    entry.set_present(true);
    entry
}

fn update_page_entry(entry: &mut PageEntry, flags: MapFlags) {
    if flags.contains(MapFlags::EXEC) {
        entry.set_no_execute(false);
    }
    if flags.contains(MapFlags::WRITE) {
        entry.set_writable(true);
    }
    if flags.contains(MapFlags::GLOBAL) {
        entry.set_global(true);
    }
    if flags.contains(MapFlags::USER) {
        entry.set_user(true);
    }
    if !flags.contains(MapFlags::NOCACHE) {
        entry.set_no_cache(false);
        entry.set_write_through(false);
    }
}
