use core::alloc::AllocError;

use chos_lib::init::ConstInit;
use chos_lib::mm::{VAddr, VFrame, VFrameRange, MapFlags};
use chos_lib::pool;
use chos_lib::pool::PoolBox;
use chos_lib::sync::Spinlock;
use intrusive_collections::{rbtree, KeyAdapter};

use crate::arch::mm::virt::map_page;
use crate::mm::phys::{alloc_pages, AllocFlags, MMPoolObjectAllocator, Page, PageBox};

struct StackAlloc {
    link: rbtree::AtomicLink,
    page: PageBox,
    base: VFrame,
}
static STACK_ALLOC_ALLOCATOR: MMPoolObjectAllocator<StackAlloc, 0> = ConstInit::INIT;
pool!(struct StackAllocPool: StackAlloc => &STACK_ALLOC_ALLOCATOR);

type StackAllocBox = PoolBox<StackAlloc, StackAllocPool>;

chos_lib::intrusive_adapter!(StackAllocAdapter = StackAllocBox: StackAlloc { link: rbtree::AtomicLink });

impl<'a> KeyAdapter<'a> for StackAllocAdapter {
    type Key = VFrame;
    fn get_key(&self, value: &'a StackAlloc) -> VFrame {
        value.base
    }
}

struct AllStacks {
    stack_tree: rbtree::RBTree<StackAllocAdapter>,
    next_base: VFrame,
}

static ALL_STACKS: Spinlock<AllStacks> = Spinlock::new(AllStacks {
    stack_tree: rbtree::RBTree::new(StackAllocAdapter::new()),
    next_base: VFrame::null(),
});

pub struct Stack {
    pub range: VFrameRange,
}

unsafe fn map_stack_unlocked(
    all_stacks: &mut AllStacks,
    page: &Page,
) -> Result<VFrame, AllocError> {
    let vbase = all_stacks.next_base;
    // Add guard page
    all_stacks.next_base = all_stacks.next_base.add((1 << page.order) + 1);
    map_page(page, vbase, MapFlags::WRITE | MapFlags::GLOBAL)?;
    Ok(vbase)
}

pub fn map_stack(page: &Page) -> Result<VFrame, AllocError> {
    let mut all_stacks = ALL_STACKS.lock();
    unsafe { map_stack_unlocked(&mut all_stacks, page) }
}

unsafe fn alloc_stack_unlocked(all_stacks: &mut AllStacks, order: u8) -> Result<Stack, AllocError> {
    let page = alloc_pages(order, AllocFlags::empty())?;
    let vbase = map_stack_unlocked(all_stacks, &page)?;
    let range = page.frame_range();
    all_stacks.stack_tree.insert(PoolBox::new(StackAlloc {
        link: rbtree::AtomicLink::new(),
        page,
        base: vbase,
    }));
    Ok(Stack {
        range: range.offset(vbase),
    })
}

pub fn alloc_stack(order: u8) -> Result<Stack, AllocError> {
    let mut all_stacks = ALL_STACKS.lock();
    unsafe { alloc_stack_unlocked(&mut all_stacks, order) }
}
