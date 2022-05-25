use core::alloc::AllocError;

use chos_config::arch::mm::virt;
use chos_lib::init::ConstInit;
use chos_lib::log::warn;
use chos_lib::mm::{MapFlags, PAddr, PFrame, VAddr, VFrame, VFrameRange};
use chos_lib::pool;
use chos_lib::pool::PoolBox;
use chos_lib::sync::Spinlock;
use intrusive_collections::{rbtree, Bound, KeyAdapter};

use super::{MemoryMapError, MemoryRegion, MemoryRegionType, PageFaultReason, PageFaultResult};
use crate::arch::early;
use crate::arch::mm::virt::map_page;
use crate::mm::phys::{alloc_pages_order, AllocFlags, MMPoolObjectAllocator, Page, PageBox};

struct StackAlloc {
    link: rbtree::AtomicLink,
    page: Option<PageBox>,
    base: VFrame,
}
static STACK_ALLOC_ALLOCATOR: MMPoolObjectAllocator<StackAlloc, 0> = ConstInit::INIT;
pool!(struct StackAllocPool: StackAlloc => &STACK_ALLOC_ALLOCATOR);

impl StackAlloc {
    fn is_guard_page(&self, addr: VAddr) -> bool {
        addr.frame_offset().0 == self.base
    }
}

type StackAllocBox = PoolBox<StackAlloc, StackAllocPool>;

chos_lib::intrusive_adapter!(StackAllocAdapter = StackAllocBox: StackAlloc { link: rbtree::AtomicLink });

impl<'a> KeyAdapter<'a> for StackAllocAdapter {
    type Key = VAddr;
    fn get_key(&self, value: &'a StackAlloc) -> VAddr {
        value.base.addr()
    }
}

struct AllStacks {
    stack_tree: rbtree::RBTree<StackAllocAdapter>,
    next_base: VFrame,
}

static ALL_STACKS: Spinlock<AllStacks> = Spinlock::new(AllStacks {
    stack_tree: rbtree::RBTree::new(StackAllocAdapter::new()),
    next_base: virt::STACK_BASE,
});

#[derive(Clone, Copy, Debug)]
pub struct Stack {
    pub range: VFrameRange,
}

fn map_stack_unlocked(
    all_stacks: &mut AllStacks,
    page: &Page,
    map: impl FnOnce(&Page, VFrame, MapFlags) -> Result<(), AllocError>,
) -> Result<VFrame, AllocError> {
    // Guard page is the first page, it is not mapped.
    let vbase = all_stacks.next_base;
    all_stacks.next_base = all_stacks.next_base.add(1 << page.order + 1);
    map(page, vbase.add(1), MapFlags::WRITE | MapFlags::GLOBAL)?;
    Ok(vbase)
}

pub fn map_kernel_stack(page: &Page) -> Result<VFrame, AllocError> {
    let mut all_stacks = ALL_STACKS.lock();
    map_stack_unlocked(&mut all_stacks, page, map_page)
}

fn do_alloc_kernel_stack(
    order: u8,
    map: impl FnOnce(&Page, VFrame, MapFlags) -> Result<(), AllocError>,
) -> Result<Stack, AllocError> {
    let page = alloc_pages_order(order, AllocFlags::empty())?;
    let mut all_stacks = ALL_STACKS.lock();
    let vbase = map_stack_unlocked(&mut all_stacks, &page, map)?;
    let range = page.frame_range();
    all_stacks.stack_tree.insert(PoolBox::new(StackAlloc {
        link: rbtree::AtomicLink::new(),
        page: Some(page),
        base: vbase,
    }));
    Ok(Stack {
        range: VFrameRange::new(vbase.add(1), vbase.add(range.frame_count() + 1)),
    })
}

pub fn alloc_kernel_stack(order: u8) -> Result<Stack, AllocError> {
    do_alloc_kernel_stack(order, map_page)
}

pub fn alloc_early_stack(order: u8) -> Result<Stack, AllocError> {
    do_alloc_kernel_stack(order, early::early_map_page)
}

pub struct StackMemoryRegion;

impl StackMemoryRegion {
    fn find_stack_for<R>(&self, vaddr: VAddr, f: impl FnOnce(&StackAlloc) -> R) -> Option<R> {
        let all_stacks = ALL_STACKS.lock();
        let cursor = all_stacks.stack_tree.upper_bound(Bound::Included(&vaddr));
        cursor.get().map(f)
    }
}

impl MemoryRegion for StackMemoryRegion {
    fn typ(&self) -> MemoryRegionType {
        MemoryRegionType::Stack
    }
    fn name(&self) -> &str {
        "kernel"
    }

    fn vaddr_range(&self) -> VFrameRange {
        let all_stacks = ALL_STACKS.lock();
        VFrameRange::new(virt::STACK_BASE, all_stacks.next_base)
    }

    fn paddr_of(&self, vaddr: VAddr) -> Option<PAddr> {
        self.find_stack_for(vaddr, |st| {
            st.page
                .as_ref()
                .map(|page| page.frame.addr() + (vaddr - st.base.addr()).as_u64())
        })
        .flatten()
    }

    fn map_paddr(&self, _: PFrame) -> Result<VFrame, MemoryMapError> {
        Err(MemoryMapError::CannotMap)
    }

    fn handle_page_fault(&self, vaddr: VAddr, _: PageFaultReason) -> PageFaultResult {
        self.find_stack_for(vaddr, |st| {
            if st.is_guard_page(vaddr) {
                Some(PageFaultResult::StackOverflow)
            } else {
                st.page.as_ref().and_then(|page| {
                    map_page(page, st.base.add(1), MapFlags::EXEC | MapFlags::GLOBAL)
                        .map_err(|err| {
                            warn!(
                                "Could not map {:#x}-{:#x} to {:#x}",
                                page.frame_range().start(),
                                page.frame_range().end(),
                                st.base.add(1),
                            );
                            err
                        })
                        .ok()
                        .map(|_| PageFaultResult::Mapped(PAddr::null()))
                })
            }
        })
        .flatten()
        .unwrap_or(PageFaultResult::NotMapped)
    }
}
