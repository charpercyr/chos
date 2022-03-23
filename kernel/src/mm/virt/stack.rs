use core::alloc::AllocError;

use chos_config::arch::mm::virt;
use chos_lib::init::ConstInit;
use chos_lib::mm::{MapFlags, VFrame, VFrameRange};
use chos_lib::pool;
use chos_lib::pool::PoolBox;
use chos_lib::sync::Spinlock;
use intrusive_collections::{rbtree, KeyAdapter};

use crate::arch::mm::virt::map_page;
use crate::early::EarlyStacks;
use crate::mm::phys::{alloc_pages, AllocFlags, MMPoolObjectAllocator, Page, PageBox};

#[derive(Debug)]
enum StackPage {
    Early,
    Normal(PageBox),
}

struct StackAlloc {
    link: rbtree::AtomicLink,
    page: StackPage,
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

fn map_stack_unlocked(all_stacks: &mut AllStacks, page: &Page) -> Result<VFrame, AllocError> {
    let vbase = all_stacks.next_base;
    // Add guard page
    all_stacks.next_base = all_stacks.next_base.add((1 << page.order) + 1);
    map_page(page, vbase, MapFlags::WRITE | MapFlags::GLOBAL)?;
    Ok(vbase)
}

pub fn map_kernel_stack(page: &Page) -> Result<VFrame, AllocError> {
    let mut all_stacks = ALL_STACKS.lock();
    map_stack_unlocked(&mut all_stacks, page)
}

pub fn alloc_kernel_stack(order: u8) -> Result<Stack, AllocError> {
    let page = alloc_pages(order, AllocFlags::empty())?;
    let mut all_stacks = ALL_STACKS.lock();
    let vbase = map_stack_unlocked(&mut all_stacks, &page)?;
    let range = page.frame_range();
    all_stacks.stack_tree.insert(PoolBox::new(StackAlloc {
        link: rbtree::AtomicLink::new(),
        page: StackPage::Normal(page),
        base: vbase,
    }));
    Ok(Stack {
        range: range.offset(vbase),
    })
}

pub unsafe fn init_kernel_stacks(core_count: usize, early_stacks: &[EarlyStacks]) {
    let mut all_stacks = ALL_STACKS.lock();
    let next_base = VFrame::new(
        early_stacks
            .iter()
            .map(|st| st.base + st.stride * (core_count as u64))
            .max()
            .unwrap_or(virt::STACK_BASE.addr()),
    );
    for st in early_stacks {
        all_stacks.stack_tree.insert(PoolBox::new(StackAlloc {
            link: rbtree::AtomicLink::new(),
            base: VFrame::new(st.base),
            page: StackPage::Early,
        }));
    }
    all_stacks.next_base = next_base;
}
