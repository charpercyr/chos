use chos_config::arch::mm::{stack, virt};
use chos_lib::arch::mm::{FrameSize4K, VAddr, PAGE_SIZE64};
use chos_lib::mm::VFrame;
use raw_alloc::AllocFlags;

use super::this_cpu_info;
use crate::arch::early::map_stack;
use crate::mm::phys::raw_alloc;

const USE_STACK_GUARD_PAGE: bool = true;

#[derive(Clone, Copy, Debug)]
pub struct Stacks {
    pub base: VAddr,
    pub size: u64,
    pub stride: u64,
}

impl Stacks {
    pub fn get_for(&self, id: usize) -> (VAddr, u64) {
        (self.base + (id as u64) * self.stride, self.size)
    }

    pub fn get_for_this_cpu(&self) -> (VAddr, u64) {
        self.get_for(this_cpu_info().id)
    }
}

static mut STACKS_BASE: VFrame<FrameSize4K> = VFrame::new(virt::STACK_BASE);

unsafe fn allocate_kernel_stack(order: u8) -> VAddr {
    let pages = raw_alloc::alloc_pages(order, AllocFlags::empty()).expect("Should not fail");
    let vaddr = STACKS_BASE;
    map_stack(vaddr, pages, 1 << order);
    STACKS_BASE = STACKS_BASE.add(1 << order);
    if USE_STACK_GUARD_PAGE {
        STACKS_BASE = STACKS_BASE.add(1);
    }
    vaddr.addr()
}

pub unsafe fn allocate_kernel_stacks_order(stack_count: usize, order: u8) -> Stacks {
    let base = STACKS_BASE;
    let stride = (PAGE_SIZE64 << order) + PAGE_SIZE64;

    for _ in 0..stack_count {
        allocate_kernel_stack(order);
    }

    Stacks {
        base: base.addr(),
        size: PAGE_SIZE64 << order,
        stride,
    }
}

pub unsafe fn allocate_kernel_stacks(stack_count: usize) -> Stacks {
    allocate_kernel_stacks_order(stack_count, stack::KERNEL_STACK_PAGE_ORDER)
}
