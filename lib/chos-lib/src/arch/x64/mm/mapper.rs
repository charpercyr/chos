use core::mem::MaybeUninit;

use super::{FrameSize1G, FrameSize2M, FrameSize4K, PAddr, PageEntry, PageTable, VAddr};
use crate::mm::*;

#[must_use = "Must flush or ignore"]
pub enum Flush<S: FrameSize> {
    All,
    PageRange(VFrameRange<S>),
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
        let (p4i, p3i, p2i, p1i, _) = vframe.addr().split();
        let p3 = alloc_cleaner.get_page_or_alloc(self.p4, self.base, flags, p4i)?;
        let p2 = alloc_cleaner.get_page_or_alloc(p3, self.base, flags, p3i)?;
        let p1 = alloc_cleaner.get_page_or_alloc(p2, self.base, flags, p2i)?;
        let entry = p1[p1i];
        if entry.present() {
            Err(MapError::AlreadyMapped)
        } else {
            alloc_cleaner.forget();
            p1[p1i] = create_page_entry(pframe.addr(), flags);
            Ok(Flush::All)
        }
    }

    unsafe fn unmap<A: FrameAllocator<FrameSize4K> + ?Sized>(
        &mut self,
        _frame: VFrame<FrameSize4K>,
        _alloc: &mut A,
    ) -> Result<Self::Flush, UnmapError<A::Error>> {
        Err(UnmapError::NotMapped)
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
        let (p4i, p3i, p2i, _, _) = vframe.addr().split();
        let p3 = alloc_cleaner.get_page_or_alloc(self.p4, self.base, flags, p4i)?;
        let p2 = alloc_cleaner.get_page_or_alloc(p3, self.base, flags, p3i)?;
        let mut entry = p2[p2i];
        if entry.present() {
            Err(MapError::AlreadyMapped)
        } else {
            alloc_cleaner.forget();
            entry = create_page_entry(pframe.addr(), flags);
            entry.set_huge_page(true);
            p2[p2i] = entry;
            Ok(Flush::All)
        }
    }

    unsafe fn unmap<A: FrameAllocator<FrameSize4K> + ?Sized>(
        &mut self,
        _frame: VFrame<FrameSize2M>,
        _alloc: &mut A,
    ) -> Result<Self::Flush, UnmapError<A::Error>> {
        Err(UnmapError::NotMapped)
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
        let (p4i, p3i, _, _, _) = vframe.addr().split();
        let p3 = alloc_cleaner.get_page_or_alloc(self.p4, self.base, flags, p4i)?;
        let mut entry = p3[p3i];
        if entry.present() {
            Err(MapError::AlreadyMapped)
        } else {
            alloc_cleaner.forget();
            entry = create_page_entry(pframe.addr(), flags);
            entry.set_huge_page(true);
            p3[p3i] = entry;
            Ok(Flush::All)
        }
    }

    unsafe fn unmap<A: FrameAllocator<FrameSize4K> + ?Sized>(
        &mut self,
        _frame: VFrame<FrameSize1G>,
        _alloc: &mut A,
    ) -> Result<Self::Flush, UnmapError<A::Error>> {
        Err(UnmapError::NotMapped)
    }
}

unsafe fn resolve_page_vaddr(base: VAddr, addr: PAddr) -> VAddr {
    VAddr::new_unchecked(addr.as_u64() + base.as_u64())
}

unsafe fn resolve_page_paddr(base: VAddr, addr: VAddr) -> PAddr {
    PAddr::new(addr.as_u64() - base.as_u64())
}

unsafe fn get_page_or_alloc<'p, S: FrameSize, A: FrameAllocator<FrameSize4K> + ?Sized>(
    table: &'p mut PageTable,
    base: VAddr,
    flags: MapFlags,
    alloc: &mut A,
    i: u16,
) -> Result<(&'p mut PageTable, bool), MapError<A::Error>> {
    let mut entry = table[i];
    let mut allocated = false;
    if !entry.present() {
        let vframe = alloc
            .alloc_frame()
            .map_err(|e| MapError::FrameAllocError(e))?;
        entry = create_page_entry(resolve_page_paddr(base, vframe.addr()), flags);
        allocated = true;
    } else {
        update_page_entry(&mut entry, flags);
    }
    table[i] = entry;
    let addr = resolve_page_vaddr(base, entry.phys_addr());
    Ok((&mut *(addr.as_u64() as *mut PageTable), allocated))
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

struct AllocCleaner<'a, A: FrameAllocator<FrameSize4K> + ?Sized, const N: usize> {
    alloc: &'a mut A,
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
    ) -> Result<&'p mut PageTable, MapError<A::Error>> {
        let mut entry = table[i];
        if !entry.present() {
            let vframe = self
                .alloc
                .alloc_frame()
                .map_err(MapError::from_frame_alloc_error)?;
            entry = create_page_entry(resolve_page_paddr(base, vframe.addr()), flags);
            self.addrs[self.n] = MaybeUninit::new(vframe);
            self.n += 1;
        } else {
            update_page_entry(&mut entry, flags);
        }
        table[i] = entry;
        let addr = resolve_page_vaddr(base, entry.phys_addr());
        Ok(&mut *addr.as_mut_ptr())
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
                if let Err(_) = self.alloc.dealloc_frame(addr) {
                    unreachable!("Dealloc failed for {:?}", addr);
                }
            }
        }
    }
}
