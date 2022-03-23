use core::alloc::AllocError;
use core::mem::{align_of, size_of, MaybeUninit};
use core::ptr::{write, write_bytes};
use core::slice::from_raw_parts_mut;

use bitflags::bitflags;
use chos_config::arch::mm::virt;
use chos_lib::arch::mm::PAGE_SIZE64;
use chos_lib::init::ConstInit;
use chos_lib::int::{log2u64, CeilDiv};
use chos_lib::log::domain_debug;
use chos_lib::mm::{PFrame, PFrameRange, PAddr};
use chos_lib::sync::spin::lock::Spinlock;
use intrusive_collections::{intrusive_adapter, linked_list, LinkedList, UnsafeMut};

use crate::config::domain;

#[derive(Debug, Clone, Copy)]
struct Metadata {
    total_pages: u64,
    meta_pages: u64,
    free_pages: u64,
    biggest_order: u8,
    bitmap_bits: u64,
}

#[derive(Debug)]
struct Region {
    link: linked_list::AtomicLink,
    flags: RegionFlags,
    meta: Metadata,
}

impl Region {
    fn base_paddr(&self) -> PAddr {
        let addr = (self as *const Self as u64) - virt::PHYSICAL_MAP_BASE.addr().as_u64();
        PAddr::new(addr)
    }

    fn contains(&self, addr: PAddr) -> bool {
        let base = self.base_paddr();
        let len = self.meta.total_pages * PAGE_SIZE64;
        addr >= base && addr.as_u64() < base.as_u64() + len
    }

    unsafe fn block_list_ptr(&mut self) -> *mut [MaybeUninit<BlockHead>] {
        let ptr: *mut Region = self;
        let ptr: *mut u8 = ptr.cast();
        let ptr = ptr.add(size_of::<Region>());
        let ptr = ptr.add(ptr.align_offset(align_of::<BlockHead>()));
        from_raw_parts_mut(ptr.cast(), self.meta.biggest_order as usize + 1)
    }

    unsafe fn block_list_uninit(&mut self) -> &mut [MaybeUninit<BlockHead>] {
        &mut *self.block_list_ptr()
    }

    unsafe fn block_list(&mut self) -> &mut [BlockHead] {
        MaybeUninit::slice_assume_init_mut(self.block_list_uninit())
    }

    unsafe fn bitmap_ptr(&mut self) -> *mut [MaybeUninit<usize>] {
        let ptr: *mut MaybeUninit<BlockHead> = self.block_list_ptr().cast();
        let ptr = ptr.add(self.meta.biggest_order as usize + 1);
        let ptr: *mut u8 = ptr.cast();
        let ptr = ptr.add(ptr.align_offset(align_of::<usize>()));
        let size = self.meta.bitmap_bits.ceil_div(size_of::<usize>() as u64) as usize;
        from_raw_parts_mut(ptr.cast(), size)
    }

    unsafe fn bitmap_uninit(&mut self) -> &mut [MaybeUninit<usize>] {
        &mut *self.bitmap_ptr()
    }

    unsafe fn bitmap(&mut self) -> &mut [usize] {
        MaybeUninit::slice_assume_init_mut(self.bitmap_uninit())
    }

    unsafe fn block_list_bitmap(&mut self) -> (&mut [BlockHead], &mut [usize]) {
        (
            MaybeUninit::slice_assume_init_mut(&mut *self.block_list_ptr()),
            MaybeUninit::slice_assume_init_mut(&mut *self.bitmap_ptr()),
        )
    }
}

intrusive_adapter!(RegionAdapter = UnsafeMut<Region>: Region { link: linked_list::AtomicLink });

struct Block {
    link: linked_list::AtomicLink,
}

intrusive_adapter!(BlockAdapter = UnsafeMut<Block>: Block { link: linked_list::AtomicLink });
struct BlockHead {
    blocks: LinkedList<BlockAdapter>,
    bitmap_offset: u64,
}

static mut REGIONS: LinkedList<RegionAdapter> = LinkedList::new(RegionAdapter::NEW);

bitflags! {
    pub struct AllocFlags: u64 {
    }
}

bitflags! {
    pub struct RegionFlags: u64 {
    }
}

fn estimate_bitmap_bits(pages: u64) -> u64 {
    if pages <= 1 {
        0
    } else {
        pages / 2 + estimate_bitmap_bits(pages / 2)
    }
}

fn calculate_meta(total_pages: u64) -> Metadata {
    // -1 because the 1st page will always be used for metadata
    let biggest_order = log2u64(total_pages - 1) as u8;

    let bitmap_bits = estimate_bitmap_bits(total_pages - 1);

    let meta_size = (size_of::<Region>() as u64).align_up(align_of::<BlockHead>() as u64)
        + ((size_of::<BlockHead>() as u64) * ((biggest_order as u64) + 1))
            .align_up(align_of::<usize>() as u64)
        + (bitmap_bits.ceil_div(size_of::<usize>() as u64));

    let meta_pages = meta_size.ceil_div(PAGE_SIZE64);

    Metadata {
        biggest_order,
        total_pages,
        meta_pages,
        free_pages: total_pages - meta_pages,
        bitmap_bits,
    }
}

pub unsafe fn add_region(frame: PFrameRange, flags: RegionFlags) {
    let paddr = frame.start().addr();
    assert!(paddr.is_page_aligned());
    let region = (paddr.as_u64() + virt::PHYSICAL_MAP_BASE.addr().as_u64()) as *mut Region;

    let total_pages = frame.frame_count();

    write(
        region,
        Region {
            link: linked_list::AtomicLink::new(),
            flags,
            meta: calculate_meta(total_pages),
        },
    );

    let region = &mut *region;

    let mut bitmap_offset = 0;
    let mut remaining_pages = region.meta.free_pages;

    {
        for block in region.block_list_uninit() {
            *block = MaybeUninit::new(BlockHead {
                blocks: LinkedList::new(BlockAdapter::new()),
                bitmap_offset,
            });
            remaining_pages /= 2;
            bitmap_offset += remaining_pages;
        }
        let bitmap = region.bitmap_uninit();
        write_bytes(bitmap.as_mut_ptr(), 0, bitmap.len());
    }

    let meta_pages = region.meta.meta_pages;
    let mut remaining_pages = region.meta.free_pages;
    let mut current_page = 0u64;
    let biggest_order = region.meta.biggest_order;
    let (blocks, bitmap) = region.block_list_bitmap();
    while remaining_pages > 0 {
        let order = log2u64(remaining_pages);
        let block_head = &mut blocks[order as usize];
        let block = ((current_page + meta_pages) * PAGE_SIZE64
            + paddr.as_u64()
            + virt::PHYSICAL_MAP_BASE.addr().as_u64()) as *mut Block;
        write(
            block,
            Block {
                link: linked_list::AtomicLink::new(),
            },
        );
        block_head.blocks.push_front(UnsafeMut::from_raw(block));

        if (order as u8) < biggest_order {
            let page_bit = (current_page >> (order + 1)) + block_head.bitmap_offset;
            let word = page_bit / (size_of::<usize>() as u64);
            let bit = page_bit % (size_of::<usize>() as u64);
            bitmap[word as usize] ^= 1 << bit;
        }

        current_page += 1 << order;
        remaining_pages -= 1 << order;
    }

    REGIONS.push_front(UnsafeMut::from_raw(region));
}

pub unsafe fn add_regions(it: impl IntoIterator<Item = (PFrameRange, RegionFlags)>) {
    for (paddr, flags) in it {
        add_region(paddr, flags)
    }
}

unsafe fn alloc_in_region(
    region: &mut Region,
    order: u8,
    flags: AllocFlags,
) -> Result<PFrame, AllocError> {
    if order > region.meta.biggest_order {
        return Err(AllocError);
    }
    let base_paddr = region.base_paddr();
    let meta = region.meta;
    let block_head = &mut region.block_list()[order as usize];
    let bitmap_offset = block_head.bitmap_offset;
    if let Some(mut block) = block_head.blocks.pop_front() {
        let ptr = block.as_mut() as *mut Block;
        write_bytes(ptr, 0xcc, 1);

        let paddr = (ptr as u64) - virt::PHYSICAL_MAP_BASE.addr().as_u64();

        if order < meta.biggest_order {
            let bitmap = region.bitmap();
            let page = (paddr - base_paddr.as_u64()) / PAGE_SIZE64 - meta.meta_pages;
            let page_bit = (page >> (order + 1)) + bitmap_offset;
            let word = page_bit / (size_of::<usize>() as u64);
            let bit = page_bit % (size_of::<usize>() as u64);
            bitmap[word as usize] ^= 1 << bit;
        }

        region.meta.free_pages -= 1 << order;

        Ok(PFrame::new_unchecked(PAddr::new(paddr)))
    } else {
        let block = alloc_in_region(region, order + 1, flags)?;
        let other_block = block.addr().as_u64() + (PAGE_SIZE64 << order);
        put_back_block(
            region,
            PFrame::new_unchecked(PAddr::new(other_block)),
            order,
        );
        Ok(block)
    }
}

unsafe fn put_back_block(region: &mut Region, pframe: PFrame, order: u8) {
    let meta = region.meta;
    let base_paddr = region.base_paddr();
    let (blocks, bitmap) = region.block_list_bitmap();
    let block = &mut blocks[order as usize];
    let ptr = (pframe.addr().as_u64() + virt::PHYSICAL_MAP_BASE.addr().as_u64()) as *mut Block;
    write(
        ptr,
        Block {
            link: linked_list::AtomicLink::new(),
        },
    );

    block.blocks.push_front(UnsafeMut::from_raw(ptr));

    let page = (pframe.addr().as_u64() - base_paddr.as_u64()) / PAGE_SIZE64 - meta.meta_pages;
    let page_bit = (page >> (order + 1)) + block.bitmap_offset;
    let word = page_bit / (size_of::<usize>() as u64);
    let bit = page_bit % (size_of::<usize>() as u64);
    bitmap[word as usize] ^= 1 << bit;

    region.meta.free_pages += 1 << order;
}

unsafe fn free_in_region(region: &mut Region, pframe: PFrame, order: u8) {
    let meta = region.meta;
    assert!(order <= meta.biggest_order, "Order is too big");
    let base_paddr = region.base_paddr();
    let (blocks, bitmap) = region.block_list_bitmap();
    let block = &mut blocks[order as usize];
    let page = (pframe.addr().as_u64() - base_paddr.as_u64()) / PAGE_SIZE64 - meta.meta_pages;
    let page_bit = (page >> (order + 1)) + block.bitmap_offset;
    let word = page_bit / (size_of::<usize>() as u64);
    let bit = page_bit % (size_of::<usize>() as u64);
    if order < meta.biggest_order && bitmap[word as usize] & (1 << bit) != 0 {
        let mut cursor = block.blocks.front_mut();
        loop {
            if let Some(b) = cursor.get() {
                let other_paddr =
                    b as *const Block as u64 - virt::PHYSICAL_MAP_BASE.addr().as_u64();
                if pframe.addr().as_u64().abs_diff(other_paddr) == (PAGE_SIZE64 << order) {
                    cursor.remove();
                    bitmap[word as usize] ^= 1 << bit;
                    let other_paddr = PAddr::new(other_paddr);
                    free_in_region(
                        region,
                        PFrame::new_unchecked(pframe.addr().min(other_paddr)),
                        order + 1,
                    );
                    break;
                }
                cursor.move_next();
            } else {
                panic!(
                    "Could not find block {:012x}",
                    pframe.addr().as_u64() + (PAGE_SIZE64 << order),
                );
            }
        }
    } else {
        put_back_block(region, pframe, order);
    }
}

pub unsafe fn alloc_pages_unlocked(order: u8, flags: AllocFlags) -> Result<PFrame, AllocError> {
    domain_debug!(
        domain::PALLOC,
        "alloc_pages(order = {}, flags = {:?})",
        order,
        flags
    );
    for region in REGIONS.iter_mut() {
        if region.meta.biggest_order >= order && region.meta.free_pages >= (1 << order) {
            if let Ok(addr) = alloc_in_region(region, order, flags) {
                return Ok(addr);
            }
        }
    }
    Err(AllocError)
}

pub unsafe fn dealloc_pages_unlocked(page: PFrame, order: u8) {
    domain_debug!(
        domain::PALLOC,
        "dealloc_pages(page = {:x}, order = {})",
        page,
        order
    );
    for region in REGIONS.iter_mut() {
        if region.contains(page.addr()) {
            free_in_region(region, page, order);
            return;
        }
    }
    panic!("Invalid address {:?}", page);
}

static ALLOC_LOCK: Spinlock<()> = Spinlock::INIT;

pub fn alloc_pages(order: u8, flags: AllocFlags) -> Result<PFrame, AllocError> {
    let _guard = ALLOC_LOCK.lock();
    unsafe { alloc_pages_unlocked(order, flags) }
}

pub unsafe fn dealloc_pages(pframe: PFrame, order: u8) {
    let _guard = ALLOC_LOCK.lock();
    dealloc_pages_unlocked(pframe, order)
}

#[derive(Debug, Clone, Copy)]
pub struct RegionInfo {
    pub range: PFrameRange,
    pub free_pages: u64,
    pub total_pages: u64,
    pub biggest_order: u8,
}

pub fn get_regions_info(mut callback: impl FnMut(RegionInfo)) {
    let _guard = ALLOC_LOCK.lock();
    for region in unsafe { &REGIONS } {
        callback(RegionInfo {
            biggest_order: region.meta.biggest_order,
            free_pages: region.meta.free_pages,
            total_pages: region.meta.total_pages,
            range: PFrameRange::new(
                unsafe { PFrame::new_unchecked(region.base_paddr()) },
                unsafe { PFrame::new_unchecked(region.base_paddr()).add(region.meta.total_pages) },
            ),
        })
    }
}
