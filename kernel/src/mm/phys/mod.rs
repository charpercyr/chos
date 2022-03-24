pub mod raw_alloc;
use core::alloc::AllocError;
use core::ptr::NonNull;

use chos_lib::arch::mm::PAGE_SIZE;
use chos_lib::init::ConstInit;
use chos_lib::mm::{PFrame, PFrameRange, VAddr, VFrame};
use chos_lib::pool::{IArc, IArcAdapter, IArcCount, Pool, PoolBox};
use chos_lib::sync::spin::lock::RawSpinLock;
use chos_lib::sync::Spinlock;
use intrusive_collections::{rbtree, KeyAdapter};
pub use raw_alloc::{add_region, add_regions, AllocFlags, RegionFlags};

use super::slab::{ObjectAllocator, PoolObjectAllocator, Slab, SlabAllocator};
use super::virt::{map_page, map_pframe, paddr_of, MemoryRegionType};

#[derive(Debug)]
pub struct Page {
    count: IArcCount,
    rb_link: rbtree::AtomicLink,
    pub frame: PFrame,
    pub order: u8,
}

impl IArcAdapter for Page {
    #[inline]
    fn count(&self) -> &IArcCount {
        &self.count
    }
}

impl Page {
    pub fn frame_range(&self) -> PFrameRange {
        PFrameRange::new(self.frame, self.frame.add(1 << self.order))
    }
}

chos_lib::intrusive_adapter!(pub PageListBoxAdapter = PageBox: Page { rb_link: rbtree::AtomicLink });
chos_lib::intrusive_adapter!(pub PageListArcAdapter = PageArc: Page { rb_link: rbtree::AtomicLink });

impl<'a> KeyAdapter<'a> for PageListBoxAdapter {
    type Key = PFrame;
    fn get_key(&self, value: &'a Page) -> PFrame {
        value.frame
    }
}
impl<'a> KeyAdapter<'a> for PageListArcAdapter {
    type Key = PFrame;
    fn get_key(&self, value: &'a Page) -> PFrame {
        value.frame
    }
}

struct PageSlab {
    vaddr: VFrame,
}

impl PageSlab {
    const ORDER: u8 = 0;
}

impl Slab for PageSlab {
    const SIZE: usize = PAGE_SIZE << Self::ORDER;

    fn frame_containing(addr: VAddr) -> VAddr {
        unsafe { VAddr::new_unchecked(((addr.as_u64() as usize) / Self::SIZE * Self::SIZE) as u64) }
    }

    fn vaddr(&self) -> VAddr {
        self.vaddr.addr()
    }
}

struct PageSlabAllocator;

unsafe impl SlabAllocator for PageSlabAllocator {
    type Slab = PageSlab;
    unsafe fn alloc_slab(&mut self) -> Result<Self::Slab, AllocError> {
        let paddr = raw_alloc::alloc_pages_unlocked(PageSlab::ORDER, AllocFlags::empty())?;
        let vaddr = map_pframe(paddr, MemoryRegionType::Alloc).map_err(|_| {
            raw_alloc::dealloc_pages_unlocked(paddr, PageSlab::ORDER);
            AllocError
        })?;
        Ok(PageSlab { vaddr })
    }
    unsafe fn dealloc_slab(&mut self, frame: Self::Slab) {
        let paddr =
            paddr_of(frame.vaddr.addr(), MemoryRegionType::Alloc).expect("Should be mapped");
        raw_alloc::dealloc_pages_unlocked(PFrame::new_unchecked(paddr), PageSlab::ORDER)
    }
}

struct PagePoolImpl {
    alloc: Spinlock<ObjectAllocator<PageSlabAllocator, Page>>,
}

impl PagePoolImpl {
    pub const fn new() -> Self {
        Self {
            alloc: Spinlock::new(ObjectAllocator::new(PageSlabAllocator)),
        }
    }
}

unsafe impl Pool<Page> for PagePoolImpl {
    unsafe fn allocate(&self) -> Result<NonNull<Page>, AllocError> {
        let mut slab = self.alloc.lock();
        slab.alloc().map(|p| p.cast())
    }

    unsafe fn deallocate(&self, ptr: NonNull<Page>) {
        let &Page {
            order,
            frame: paddr,
            ..
        } = ptr.as_ref();
        let mut slab = self.alloc.lock();
        slab.dealloc(ptr);
        raw_alloc::dealloc_pages(paddr, order);
    }
}

static PAGE_POOL: PagePoolImpl = PagePoolImpl::new();
chos_lib::pool!(pub struct PagePool: Page => &PAGE_POOL);

pub type PageBox = PoolBox<Page, PagePool>;
pub type PageArc = IArc<Page, PagePool>;

pub fn alloc_pages(order: u8, flags: AllocFlags) -> Result<PageBox, AllocError> {
    let paddr = raw_alloc::alloc_pages(order, flags)?;
    PoolBox::try_new(Page {
        count: IArcCount::INIT,
        rb_link: rbtree::AtomicLink::new(),
        frame: paddr,
        order,
    })
    .map_err(|e| {
        unsafe { raw_alloc::dealloc_pages(paddr, order) };
        e
    })
}

pub struct MMSlab<const O: u8> {
    page: PageBox,
    vaddr: VAddr,
}

impl<const O: u8> Slab for MMSlab<O> {
    const SIZE: usize = PAGE_SIZE << O;
    fn frame_containing(addr: VAddr) -> VAddr {
        unsafe { VAddr::new_unchecked(addr.as_u64() / (Self::SIZE as u64) * (Self::SIZE as u64)) }
    }
    fn vaddr(&self) -> VAddr {
        self.vaddr
    }
}

pub struct MMSlabAllocator<const O: u8>;

impl<const O: u8> ConstInit for MMSlabAllocator<O> {
    const INIT: Self = Self;
}

unsafe impl<const O: u8> SlabAllocator for MMSlabAllocator<O> {
    type Slab = MMSlab<O>;
    unsafe fn alloc_slab(&mut self) -> Result<Self::Slab, AllocError> {
        let page = alloc_pages(O, AllocFlags::empty())?;
        let vaddr = map_page(&page, MemoryRegionType::Normal).map_err(|_| AllocError)?;
        Ok(MMSlab {
            page,
            vaddr: vaddr.addr(),
        })
    }
    unsafe fn dealloc_slab(&mut self, frame: Self::Slab) {
        drop(frame)
    }
}

pub type MMPoolObjectAllocator<T, const O: u8> =
    PoolObjectAllocator<RawSpinLock, MMSlabAllocator<O>, T>;
