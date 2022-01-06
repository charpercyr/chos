pub mod raw_alloc;
use core::alloc::AllocError;
use core::ptr::NonNull;

use chos_lib::arch::mm::{PAddr, VAddr, PAGE_SIZE};
use chos_lib::init::ConstInit;
use chos_lib::pool::{IArc, IArcAdapter, IArcCount, Pool, PoolBox};
use chos_lib::sync::fake::FakeLock;
use chos_lib::sync::lock::Lock;
use chos_lib::sync::spin::lock::RawSpinLock;
use intrusive_collections::linked_list;
pub use raw_alloc::{add_region, add_regions, AllocFlags, RegionFlags};

use super::slab::{ObjectAllocator, PoolObjectAllocator, Slab, SlabAllocator};
use super::virt::{map_paddr, map_page, paddr_of, MemoryRegion};

#[derive(Debug)]
pub struct Page {
    count: IArcCount,
    list_link: linked_list::AtomicLink,
    pub paddr: PAddr,
    pub order: u8,
}

impl IArcAdapter for Page {
    #[inline]
    fn count(&self) -> &IArcCount {
        &self.count
    }
}

chos_lib::intrusive_adapter!(pub PageListBoxAdapter = PageBox: Page { list_link: linked_list::AtomicLink });
chos_lib::intrusive_adapter!(pub PageListArcAdapter = PageArc: Page { list_link: linked_list::AtomicLink });

struct PageSlab {
    vaddr: VAddr,
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
        self.vaddr
    }
}

struct PageSlabAllocator;

unsafe impl SlabAllocator for PageSlabAllocator {
    type Slab = PageSlab;
    unsafe fn alloc_slab(&mut self) -> Result<Self::Slab, AllocError> {
        let paddr = raw_alloc::alloc_pages_unlocked(PageSlab::ORDER, AllocFlags::empty())?;
        let vaddr = map_paddr(paddr, MemoryRegion::Alloc).map_err(|_| {
            raw_alloc::dealloc_pages_unlocked(paddr, PageSlab::ORDER);
            AllocError
        })?;
        Ok(PageSlab { vaddr })
    }
    unsafe fn dealloc_slab(&mut self, frame: Self::Slab) {
        let paddr = paddr_of(frame.vaddr, MemoryRegion::Alloc).expect("Should be mapped");
        raw_alloc::dealloc_pages_unlocked(paddr, PageSlab::ORDER)
    }
}

struct PagePoolImpl {
    alloc: Lock<FakeLock, ObjectAllocator<PageSlabAllocator, Page>>,
}

impl PagePoolImpl {
    pub const unsafe fn new() -> Self {
        Self {
            alloc: Lock::new_with(ObjectAllocator::new(PageSlabAllocator), FakeLock::new()),
        }
    }
}

unsafe impl Pool<Page> for PagePoolImpl {
    unsafe fn allocate(&self) -> Result<NonNull<Page>, AllocError> {
        let mut slab = self.alloc.lock();
        slab.alloc().map(|p| p.cast())
    }

    unsafe fn deallocate(&self, ptr: NonNull<Page>) {
        let &Page { order, paddr, .. } = ptr.as_ref();
        let mut slab = self.alloc.lock();
        slab.dealloc(ptr);
        raw_alloc::dealloc_pages(paddr, order);
    }
}

static PAGE_POOL: PagePoolImpl = unsafe { PagePoolImpl::new() };
chos_lib::pool!(pub struct PagePool: Page => &PAGE_POOL);

pub type PageBox = PoolBox<Page, PagePool>;
pub type PageArc = IArc<Page, PagePool>;

pub unsafe fn alloc_pages(order: u8, flags: AllocFlags) -> Result<PageBox, AllocError> {
    let paddr = raw_alloc::alloc_pages(order, flags)?;
    PoolBox::try_new(Page {
        count: IArcCount::INIT,
        list_link: linked_list::AtomicLink::new(),
        paddr,
        order,
    })
    .map_err(|e| {
        raw_alloc::dealloc_pages(paddr, order);
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
        let vaddr = map_page(&page, MemoryRegion::Normal).map_err(|_| AllocError)?;
        Ok(MMSlab { page, vaddr })
    }
    unsafe fn dealloc_slab(&mut self, frame: Self::Slab) {
        drop(frame)
    }
}

pub type MMPoolObjectAllocator<T, const O: u8> =
    PoolObjectAllocator<RawSpinLock, MMSlabAllocator<O>, T>;
