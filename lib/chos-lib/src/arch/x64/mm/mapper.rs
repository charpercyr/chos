use core::arch::asm;
use core::mem::MaybeUninit;

use super::{FrameSize1G, FrameSize2M, FrameSize4K, PAddr, PageEntry, PageTable, VAddr};
use crate::mm::*;

const FLUSH_MAX_INVLPG_FRAMES: u64 = 11;

#[must_use = "Must flush or ignore"]
pub enum Flush<S: FrameSize> {
    All,
    Range(VFrameRange<S>),
    None,
}

impl<S: FrameSize> MapperFlush for Flush<S> {
    const NONE: Self = Self::None;

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
            Self::Range(range) => unsafe {
                for vframe in range {
                    asm! {
                        "invlpg ({addr})",
                        addr = in(reg) vframe.addr().as_u64(),
                        options(att_syntax, nostack),
                    }
                }
            },
            Self::None => (),
        }
    }

    fn combine(self, rhs: Self) -> Self {
        match (self, rhs) {
            (Self::All, _) | (_, Self::All) => Self::All,
            (Self::Range(r1), Self::Range(r2)) => {
                if let Some(r) = r1.intersection(r2) {
                    if r.frame_count() <= FLUSH_MAX_INVLPG_FRAMES {
                        Self::Range(r)
                    } else {
                        Self::All
                    }
                } else {
                    Self::All
                }
            }
            (Self::None, flush) | (flush, Self::None) => flush,
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
    type PGTFrameSize = FrameSize4K;
    unsafe fn map<A: FrameAllocator<FrameSize4K> + ?Sized>(
        &mut self,
        pframe: PFrame<FrameSize4K>,
        vframe: VFrame<FrameSize4K>,
        flags: MapFlags,
        alloc: &mut A,
    ) -> Result<Self::Flush, MapError<A::Error>> {
        let mut alloc_cleaner = AllocCleaner::<A, 3>::new(alloc);
        let (p4i, p3i, p2i, p1i) = vframe.split();
        let (p3, p3_a) = alloc_cleaner.get_page_or_alloc(self.p4, self.base, flags, p4i)?;
        let (p2, p2_a) = alloc_cleaner.get_page_or_alloc(p3, self.base, flags, p3i)?;
        let (p1, _) = alloc_cleaner.get_page_or_alloc(p2, self.base, flags, p2i)?;
        let entry = p1[p1i];
        if entry.present() {
            Err(MapError::AlreadyMapped)
        } else {
            alloc_cleaner.forget();
            p1[p1i] = create_page_entry(pframe.addr(), flags);
            inc_child_alloc_count(&mut p2[p2i]);
            if p2_a {
                inc_child_alloc_count(&mut p3[p3i]);
            }
            if p3_a {
                inc_child_alloc_count(&mut self.p4[p4i]);
            }
            Ok(Flush::All)
        }
    }

    unsafe fn unmap<A: FrameAllocator<FrameSize4K> + ?Sized>(
        &mut self,
        _frame: VFrame<FrameSize4K>,
        _alloc: &mut A,
    ) -> Result<Self::Flush, UnmapError<A::Error>> {
        todo!("Unmap(4K)")
    }
}

impl Mapper<FrameSize2M> for OffsetMapper<'_> {
    type Flush = Flush<FrameSize2M>;
    type PGTFrameSize = FrameSize4K;
    unsafe fn map<A: FrameAllocator<FrameSize4K> + ?Sized>(
        &mut self,
        pframe: PFrame<FrameSize2M>,
        vframe: VFrame<FrameSize2M>,
        flags: MapFlags,
        alloc: &mut A,
    ) -> Result<Self::Flush, MapError<A::Error>> {
        let mut alloc_cleaner = AllocCleaner::<A, 2>::new(alloc);
        let (p4i, p3i, p2i) = vframe.split();
        let (p3, p3_a) = alloc_cleaner.get_page_or_alloc(self.p4, self.base, flags, p4i)?;
        let (p2, _) = alloc_cleaner.get_page_or_alloc(p3, self.base, flags, p3i)?;
        let mut entry = p2[p2i];
        if entry.present() {
            Err(MapError::AlreadyMapped)
        } else {
            alloc_cleaner.forget();
            entry = create_page_entry(pframe.addr(), flags);
            entry.set_huge_page(true);
            p2[p2i] = entry;
            inc_child_alloc_count(&mut p3[p3i]);
            if p3_a {
                inc_child_alloc_count(&mut self.p4[p4i]);
            }
            Ok(Flush::All)
        }
    }

    unsafe fn unmap<A: FrameAllocator<FrameSize4K> + ?Sized>(
        &mut self,
        _frame: VFrame<FrameSize2M>,
        _alloc: &mut A,
    ) -> Result<Self::Flush, UnmapError<A::Error>> {
        todo!("Unmap(2M)")
    }
}

impl Mapper<FrameSize1G> for OffsetMapper<'_> {
    type Flush = Flush<FrameSize1G>;
    type PGTFrameSize = FrameSize4K;
    unsafe fn map<A: FrameAllocator<FrameSize4K> + ?Sized>(
        &mut self,
        pframe: PFrame<FrameSize1G>,
        vframe: VFrame<FrameSize1G>,
        flags: MapFlags,
        alloc: &mut A,
    ) -> Result<Self::Flush, MapError<A::Error>> {
        let mut alloc_cleaner = AllocCleaner::<A, 1>::new(alloc);
        let (p4i, p3i) = vframe.split();
        let (p3, _) = alloc_cleaner.get_page_or_alloc(self.p4, self.base, flags, p4i)?;
        let mut entry = p3[p3i];
        if entry.present() {
            Err(MapError::AlreadyMapped)
        } else {
            alloc_cleaner.forget();
            entry = create_page_entry(pframe.addr(), flags);
            entry.set_huge_page(true);
            p3[p3i] = entry;
            inc_child_alloc_count(&mut self.p4[p4i]);
            Ok(Flush::All)
        }
    }

    unsafe fn unmap<A: FrameAllocator<FrameSize4K> + ?Sized>(
        &mut self,
        frame: VFrame<FrameSize1G>,
        alloc: &mut A,
    ) -> Result<Self::Flush, UnmapError<A::Error>> {
        let (p4i, p3i) = frame.split();
        let p3 = get_page_table(self.p4, self.base, p4i).ok_or(UnmapError::NotMapped)?;
        let entry = &mut p3[p3i];
        if !entry.present() {
            return Err(UnmapError::NotMapped);
        }
        if !entry.huge_page() {
            return Err(UnmapError::InvalidSize);
        }
        *entry = PageEntry::new();
        if dec_child_alloc_count(&mut self.p4[p4i]) {
            alloc
                .dealloc_frame(VFrame::new_unchecked(resolve_page_vaddr(
                    self.base,
                    self.p4[p4i].paddr(),
                )))
                .map_err(UnmapError::FrameAllocError)?;
            self.p4[p4i] = PageEntry::new();
        }
        Ok(Flush::Range(VFrameRange::new(frame, frame.add(1))))
    }
}

impl<S: FrameSize> RangeMapper<S> for OffsetMapper<'_>
where
    Self: Mapper<S, Flush = Flush<S>>,
{
    unsafe fn map_range<A: FrameAllocator<Self::PGTFrameSize> + ?Sized>(
        &mut self,
        prange: PFrameRange<S>,
        vbase: VFrame<S>,
        flags: MapFlags,
        alloc: &mut A,
    ) -> Result<Self::Flush, MapError<A::Error>> {
        let pbase = prange.start();
        for pframe in prange {
            let vframe = VFrame::<S>::new(vbase.addr() + pframe.addr() - pbase.addr());
            self.map(pframe, vframe, flags, alloc)?.ignore();
        }
        Ok(Flush::None)
    }
    unsafe fn unmap_range<A: FrameAllocator<Self::PGTFrameSize> + ?Sized>(
        &mut self,
        vrange: VFrameRange<S>,
        alloc: &mut A,
    ) -> Result<Self::Flush, UnmapError<A::Error>> {
        for vframe in vrange {
            self.unmap(vframe, alloc)?.ignore();
        }
        if vrange.frame_count() > FLUSH_MAX_INVLPG_FRAMES {
            Ok(Flush::All)
        } else {
            Ok(Flush::Range(vrange))
        }
    }
}

impl PAddrResolver for OffsetMapper<'_> {
    fn paddr_of(&self, vaddr: VAddr) -> Option<PAddr> {
        let (p4i, p3i, p2i, p1i, off) = vaddr.split();
        unsafe {
            let p3 = page_table_from_entry(self.base, self.p4[p4i])?;
            let p2 = match (p3[p3i].present(), p3[p3i].huge_page()) {
                (true, true) => {
                    return Some(
                        p3[p3i].paddr() + ((p2i as u64) << 21) + ((p1i as u64) << 12) + off as u64,
                    )
                }
                _ => page_table_from_entry(self.base, p3[p3i])?,
            };
            let p1 = match (p2[p2i].present(), p2[p2i].huge_page()) {
                (true, true) => return Some(p2[p2i].paddr() + ((p1i as u64) << 12) + off as u64),
                _ => page_table_from_entry(self.base, p2[p2i])?,
            };
            p1[p1i].present().then(|| p1[p1i].paddr() + off as u64)
        }
    }
}

unsafe fn resolve_page_vaddr(base: VAddr, addr: PAddr) -> VAddr {
    VAddr::new_unchecked(addr.as_u64() + base.as_u64())
}

unsafe fn resolve_page_paddr(base: VAddr, addr: VAddr) -> PAddr {
    PAddr::new(addr.as_u64() - base.as_u64())
}

unsafe fn get_page_table(table: &mut PageTable, base: VAddr, i: u16) -> Option<&mut PageTable> {
    let entry = table[i];
    entry
        .present()
        .then(|| resolve_page_vaddr(base, entry.paddr()).as_mut())
}

unsafe fn get_page_table_or_alloc<'p, S: FrameSize, A: FrameAllocator<FrameSize4K> + ?Sized>(
    table: &'p mut PageTable,
    base: VAddr,
    flags: MapFlags,
    alloc: &mut A,
    i: u16,
) -> Result<(&'p mut PageTable, bool), MapError<A::Error>> {
    let mut entry = table[i];
    let mut allocated = false;
    if !entry.present() {
        let vframe = alloc.alloc_frame().map_err(MapError::FrameAllocError)?;
        entry = create_page_entry(resolve_page_paddr(base, vframe.addr()), flags);
        allocated = true;
    } else {
        update_page_entry(&mut entry, flags);
    }
    table[i] = entry;
    let addr = resolve_page_vaddr(base, entry.paddr());
    Ok((addr.as_mut(), allocated))
}

fn create_page_entry(paddr: PAddr, flags: MapFlags) -> PageEntry {
    let mut entry = PageEntry::zero();
    entry.set_paddr(paddr);
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

fn inc_child_alloc_count(entry: &mut PageEntry) {
    let new = entry.child_alloc_count() + 1;
    entry.set_child_alloc_count(new);
}

fn dec_child_alloc_count(entry: &mut PageEntry) -> bool {
    let new = entry.child_alloc_count() - 1;
    entry.set_child_alloc_count(new);
    new == 0
}

struct AllocCleaner<'alloc, A: FrameAllocator<FrameSize4K> + ?Sized, const N: usize> {
    alloc: &'alloc mut A,
    addrs: [MaybeUninit<VFrame<FrameSize4K>>; N],
    n: usize,
}

impl<'a, A: FrameAllocator<FrameSize4K> + ?Sized, const N: usize> AllocCleaner<'a, A, N> {
    const fn new(alloc: &'a mut A) -> Self {
        Self {
            alloc,
            addrs: [MaybeUninit::uninit(); N],
            n: 0,
        }
    }

    unsafe fn get_page_or_alloc<'p>(
        &mut self,
        table: &'p mut PageTable,
        base: VAddr,
        flags: MapFlags,
        i: u16,
    ) -> Result<(&'p mut PageTable, bool), MapError<A::Error>> {
        let entry = &mut table[i];
        let allocated;
        if !entry.present() {
            let vframe = self
                .alloc
                .alloc_frame()
                .map_err(MapError::from_frame_alloc_error)?;
            *entry = create_page_entry(resolve_page_paddr(base, vframe.addr()), flags);
            self.addrs[self.n] = MaybeUninit::new(vframe);
            self.n += 1;
            allocated = true;
        } else {
            update_page_entry(entry, flags);
            allocated = false;
        }
        let addr = resolve_page_vaddr(base, entry.paddr());
        Ok((addr.as_mut(), allocated))
    }

    fn forget(self) {
        core::mem::forget(self)
    }
}

impl<'a, A: FrameAllocator<FrameSize4K> + ?Sized, const N: usize> Drop for AllocCleaner<'a, A, N> {
    fn drop(&mut self) {
        for i in 0..self.n {
            unsafe {
                let addr = self.addrs[i].assume_init();
                if self.alloc.dealloc_frame(addr).is_err() {
                    unreachable!("Dealloc failed for {:?}", addr);
                }
            }
        }
    }
}

unsafe fn page_table_from_entry(base: VAddr, entry: PageEntry) -> Option<&'static PageTable> {
    entry.present().then(|| (base + entry.paddr()).as_ref())
}
