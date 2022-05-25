pub mod raw_alloc;
use core::alloc::{AllocError, Layout};
use core::ptr::NonNull;

use chos_lib::arch::mm::{FrameSize4K, PAGE_SIZE};
use chos_lib::init::ConstInit;
use chos_lib::int::log2u64;
use chos_lib::mm::{PFrame, PFrameRange, VAddr, VFrame, VFrameRange};
use chos_lib::pool::{iarc_adapter, IArc, IArcCount, Pool, PoolBox};
use chos_lib::sync::spin::lock::RawSpinLock;
use chos_lib::sync::Spinlock;
use intrusive_collections::{linked_list, rbtree, Bound, KeyAdapter};
pub use raw_alloc::{add_region, add_regions, AllocFlags, RegionFlags};

use super::slab::{ObjectAllocator, PoolObjectAllocator, Slab, SlabAllocator};
use super::virt::{map_page, map_pframe, paddr_of, MemoryRegionType};

#[derive(Debug)]
pub struct Page {
    count: IArcCount,
    rb_link: rbtree::AtomicLink,
    pub frame: PFrame,
    pub vframe: Option<VFrame>,
    pub order: u8,
}
iarc_adapter!(Page: count);

impl Page {
    pub fn frame_range(&self) -> PFrameRange {
        PFrameRange::new(self.frame, self.frame.add(1 << self.order))
    }

    pub fn vframe_range(&self) -> Option<VFrameRange> {
        self.vframe.map(|vframe| self.frame_range().offset(vframe))
    }
}

chos_lib::intrusive_adapter!(pub PageBoxAdapter = PageBox: Page { rb_link: rbtree::AtomicLink });
chos_lib::intrusive_adapter!(pub PageArcAdapter = PageArc: Page { rb_link: rbtree::AtomicLink });

impl<'a> KeyAdapter<'a> for PageBoxAdapter {
    type Key = PFrame;
    fn get_key(&self, value: &'a Page) -> PFrame {
        value.frame
    }
}
impl<'a> KeyAdapter<'a> for PageArcAdapter {
    type Key = PFrame;
    fn get_key(&self, value: &'a Page) -> PFrame {
        value.frame
    }
}

struct PageSlab {
    vframe: VFrame,
}

impl PageSlab {
    const ORDER: u8 = 0;
}

impl Slab for PageSlab {
    const SIZE: usize = PAGE_SIZE << Self::ORDER;

    fn vaddr(&self) -> VAddr {
        self.vframe.addr()
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
        Ok(PageSlab { vframe: vaddr })
    }
    unsafe fn dealloc_slab(&mut self, frame: Self::Slab) {
        let paddr =
            paddr_of(frame.vframe.addr(), MemoryRegionType::Alloc).expect("Should be mapped");
        raw_alloc::dealloc_pages_unlocked(PFrame::new_unchecked(paddr), PageSlab::ORDER)
    }
    fn frame_containing(&mut self, addr: VAddr) -> Option<VAddr> {
        VFrame::<FrameSize4K>::new_align_down(addr).addr().into() // OK since the order is 0
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

    unsafe fn deallocate(&self, ptr: NonNull<Page>, _: Layout) {
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

pub fn alloc_pages_order(order: u8, flags: AllocFlags) -> Result<PageBox, AllocError> {
    let paddr = raw_alloc::alloc_pages(order, flags)?;
    PoolBox::try_new(Page {
        count: IArcCount::INIT,
        rb_link: rbtree::AtomicLink::new(),
        frame: paddr,
        vframe: None,
        order,
    })
    .map_err(|e| {
        unsafe { raw_alloc::dealloc_pages(paddr, order) };
        e
    })
}

pub fn alloc_pages(
    mut count: usize,
    flags: AllocFlags,
) -> Result<linked_list::LinkedList<PageBoxAdapter>, AllocError> {
    let mut list = linked_list::LinkedList::new(PageBoxAdapter::new());
    while count > 0 {
        let order = log2u64(count as u64);
        let page = alloc_pages_order(order as u8, flags)?;
        list.push_back(page);
        count -= 1 << order;
    }
    Ok(list)
}

pub struct MMSlab<const O: u8> {
    page: PageArc,
}

impl<const O: u8> Slab for MMSlab<O> {
    const SIZE: usize = PAGE_SIZE << O;
    fn vaddr(&self) -> VAddr {
        self.page.vframe.unwrap().addr()
    }
}

pub struct MMSlabAllocator<const O: u8> {
    all_pages: rbtree::RBTree<PageArcAdapter>,
}

impl<const O: u8> ConstInit for MMSlabAllocator<O> {
    const INIT: Self = Self {
        all_pages: rbtree::RBTree::new(PageArcAdapter::NEW),
    };
}

unsafe impl<const O: u8> SlabAllocator for MMSlabAllocator<O> {
    type Slab = MMSlab<O>;
    unsafe fn alloc_slab(&mut self) -> Result<Self::Slab, AllocError> {
        let mut page = alloc_pages_order(O, AllocFlags::empty())?;
        let vframe = map_page(&page, MemoryRegionType::Normal).map_err(|_| AllocError)?;
        page.vframe = Some(vframe);
        let page: PageArc = page.into();
        self.all_pages.insert(page.clone());
        Ok(MMSlab { page })
    }
    unsafe fn dealloc_slab(&mut self, frame: Self::Slab) {
        drop(frame)
    }

    fn frame_containing(&mut self, addr: VAddr) -> Option<VAddr> {
        let pframe = PFrame::new_align_down(paddr_of(addr, MemoryRegionType::Normal)?);
        let cur = self.all_pages.lower_bound(Bound::Included(&pframe));
        cur.get().map(|page| page.vframe.unwrap().addr())
    }
}

pub type MMPoolObjectAllocator<T, const O: u8> =
    PoolObjectAllocator<RawSpinLock, MMSlabAllocator<O>, T>;
