use alloc::alloc::{GlobalAlloc, Layout};
use core::mem::align_of;
use core::ptr::null_mut;
use core::slice;

use chos_lib::arch::mm::PAGE_SHIFT;
use chos_lib::int::ceil_log2u64;
use chos_lib::log::domain_debug;
use chos_lib::mm::{PFrame, VAddr, VFrame};
use chos_lib::sync::spin::lock::Spinlock;

use super::phys::MMSlabAllocator;
use super::slab::RawObjectAllocator;
use crate::config::domain;
use crate::mm::phys::raw_alloc::{self, AllocFlags};
use crate::mm::virt::{map_pframe, paddr_of};

macro_rules! kalloc_sizes {
    (@array_len) => {
        0
    };
    (@array_len $size:expr, $($rest:tt)*) => {
        1 + kalloc_sizes!(@array_len $($rest)*)
    };
    ($name:ident = [$($size:expr => $order:expr),* $(,)?]) => {
        static $name: [(usize, &'static (dyn KAllocSize + Send + Sync)); kalloc_sizes!(@array_len $($size,)*)] = [
            $((
                $size,
                {
                    static ALLOC: Spinlock<RawObjectAllocator<MMSlabAllocator<$order>>> = Spinlock::new(RawObjectAllocator::new(
                        <MMSlabAllocator::<$order> as chos_lib::init::ConstInit>::INIT,
                        unsafe { Layout::from_size_align_unchecked($size, align_of::<usize>()) },
                    ));
                    &ALLOC
                },
            ),)*
        ];
    };
}

unsafe trait KAllocSize {
    unsafe fn alloc(&self) -> *mut u8;
    unsafe fn dealloc(&self, ptr: *mut u8);
}

unsafe impl<const O: u8> KAllocSize for Spinlock<RawObjectAllocator<MMSlabAllocator<O>>> {
    unsafe fn alloc(&self) -> *mut u8 {
        let mut alloc = self.lock();
        alloc.alloc().unwrap().as_mut().as_mut_ptr()
    }
    unsafe fn dealloc(&self, ptr: *mut u8) {
        let mut alloc = self.lock();
        let slice = slice::from_raw_parts_mut(ptr, alloc.layout().size());
        alloc.dealloc(slice.into())
    }
}

kalloc_sizes!(
    KALLOC_SIZES = [
        8  => 0,
        16 => 0,
        32 => 0,
        48 => 0,
        64 => 0,
        96 => 0,
        128 => 0,
        192 => 0,
        256 => 1,
        512 => 1,
        1024 => 2,
        2048 => 4,
    ]
);

struct KAlloc;

unsafe impl GlobalAlloc for KAlloc {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        if layout.size() == 0 {
            return layout.align() as _;
        }
        assert!(
            layout.align() <= align_of::<usize>(),
            "Invalid alignment, use specialized slab allocator"
        );
        for &(s, alloc) in &KALLOC_SIZES {
            if s >= layout.size() {
                let ptr = alloc.alloc();
                domain_debug!(
                    domain::GLOBAL_ALLOC,
                    "alloc(size={}, align={}) = {:p}",
                    layout.size(),
                    layout.align(),
                    ptr,
                );
                return ptr;
            }
        }
        let order = ceil_log2u64(layout.size() as u64) - PAGE_SHIFT;
        let ptr = raw_alloc::alloc_pages(order as u8, AllocFlags::empty())
            .map(|paddr| {
                let vaddr = map_pframe(paddr, crate::mm::virt::MemoryRegionType::Normal)
                    .unwrap_or(VFrame::null());
                vaddr.addr().as_mut_ptr()
            })
            .unwrap_or(null_mut());
        domain_debug!(
            domain::GLOBAL_ALLOC,
            "alloc(size={}, align={}) = {:p}",
            layout.size(),
            layout.align(),
            ptr,
        );
        ptr
    }
    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        if layout.size() == 0 {
            return;
        }
        domain_debug!(
            domain::GLOBAL_ALLOC,
            "dealloc(ptr={:p}, size={}, align={})",
            ptr,
            layout.size(),
            layout.align()
        );
        assert!(
            layout.align() <= align_of::<usize>(),
            "Invalid alignment, use specialized slab allocator"
        );
        for &(s, alloc) in &KALLOC_SIZES {
            if s >= layout.size() {
                return alloc.dealloc(ptr);
            }
        }
        let order = ceil_log2u64(layout.size() as u64) - PAGE_SHIFT;
        let paddr = paddr_of(
            VAddr::new(ptr as u64),
            crate::mm::virt::MemoryRegionType::Normal,
        )
        .expect("Should exist");
        raw_alloc::dealloc_pages(PFrame::new(paddr), order as u8);
    }
}

#[global_allocator]
static KALLOC: KAlloc = KAlloc;

#[lang = "oom"]
fn out_of_memory_handler(layout: Layout) -> ! {
    panic!(
        "Out of memory, tried to allocate {} bytes (align = {})",
        layout.size(),
        layout.align()
    );
}
