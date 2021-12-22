pub mod raw_alloc;
use core::alloc::AllocError;
use core::ptr::NonNull;

use chos_config::arch::mm::virt::{self, PHYSICAL_MAP_BASE};
use chos_lib::arch::mm::{PAddr, VAddr, PAGE_SIZE};
use chos_lib::init::ConstInit;
use chos_lib::pool::{IArc, IArcAdapter, IArcCount, Pool, PoolBox};
use chos_lib::sync::fake::FakeLock;
use chos_lib::sync::lock::Lock;
use chos_lib::sync::spin::lock::{RawSpinLock, Spinlock};
use intrusive_collections::linked_list;
pub use raw_alloc::{add_region, add_regions, AllocFlags, RegionFlags};

use super::slab::{ObjectAllocator, PoolObjectAllocator, Slab, SlabAllocator};

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
        raw_alloc::alloc_pages(PageSlab::ORDER, AllocFlags::empty()).map(|p| PageSlab {
            vaddr: VAddr::new_unchecked(p.as_u64() + PHYSICAL_MAP_BASE.as_u64()),
        })
    }
    unsafe fn dealloc_slab(&mut self, frame: Self::Slab) {
        raw_alloc::dealloc_pages(
            PAddr::new(frame.vaddr.as_u64() - PHYSICAL_MAP_BASE.as_u64()),
            PageSlab::ORDER,
        )
    }
}

struct PagePoolImpl {
    alloc: Lock<FakeLock, ObjectAllocator<PageSlabAllocator, Page>>,
}

impl PagePoolImpl {
    pub const unsafe fn new() -> Self {
        Self {
            alloc: Lock::new_with_lock(ObjectAllocator::new(PageSlabAllocator), FakeLock::new()),
        }
    }
}

unsafe impl Pool<Page> for PagePoolImpl {
    unsafe fn allocate(&self) -> Result<NonNull<Page>, AllocError> {
        let mut slab = self.alloc.lock();
        slab.alloc().map(|p| p.cast())
    }

    unsafe fn deallocate(&self, ptr: NonNull<Page>) {
        let _guard = ALLOC_LOCK.lock();
        let &Page { order, paddr, .. } = ptr.as_ref();
        let mut slab = self.alloc.lock();
        slab.dealloc(ptr);
        raw_alloc::dealloc_pages(paddr, order);
    }
}

static PAGE_POOL: PagePoolImpl = unsafe { PagePoolImpl::new() };
static ALLOC_LOCK: Spinlock<()> = Spinlock::INIT;
chos_lib::pool!(pub struct PagePool: Page => &PAGE_POOL);

pub type PageBox = PoolBox<Page, PagePool>;
pub type PageArc = IArc<Page, PagePool>;

pub unsafe fn alloc_pages(order: u8, flags: AllocFlags) -> Result<PageBox, AllocError> {
    let _guard = ALLOC_LOCK.lock();
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
}

impl<const O: u8> Slab for MMSlab<O> {
    const SIZE: usize = PAGE_SIZE << O;
    fn frame_containing(addr: VAddr) -> VAddr {
        unsafe { VAddr::new_unchecked(addr.as_u64() / (Self::SIZE as u64) * (Self::SIZE as u64)) }
    }
    fn vaddr(&self) -> VAddr {
        self.page.paddr + virt::PHYSICAL_MAP_BASE
    }
}

pub struct MMSlabAllocator<const O: u8>;

impl<const O: u8> ConstInit for MMSlabAllocator<O> {
    const INIT: Self = Self;
}

unsafe impl<const O: u8> SlabAllocator for MMSlabAllocator<O> {
    type Slab = MMSlab<O>;
    unsafe fn alloc_slab(&mut self) -> Result<Self::Slab, AllocError> {
        Ok(MMSlab {
            page: alloc_pages(O, AllocFlags::empty())?,
        })
    }
    unsafe fn dealloc_slab(&mut self, frame: Self::Slab) {
        drop(frame)
    }
}

pub type MMPoolObjectAllocator<T, const O: u8> =
    PoolObjectAllocator<RawSpinLock, MMSlabAllocator<O>, T>;
