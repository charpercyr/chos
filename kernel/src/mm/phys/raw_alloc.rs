use core::alloc::AllocError;
use core::mem::{align_of, size_of, MaybeUninit};
use core::ptr::{write, write_bytes};
use core::slice::from_raw_parts_mut;

use bitflags::bitflags;
use chos_config::arch::mm::virt;
use chos_lib::arch::mm::{PAddr, PAGE_SIZE64};
use chos_lib::int::{log2u64, CeilDiv};
use chos_lib::intrusive::{list, UnsafeRef};
use chos_lib::intrusive_adapter;

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
    link: list::Link<()>,
    flags: RegionFlags,
    meta: Metadata,
}

impl Region {
    fn base_paddr(&self) -> PAddr {
        let addr = (self as *const Self as u64) - virt::PHYSICAL_MAP_BASE.as_u64();
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

intrusive_adapter!(struct RegionAdapter = UnsafeRef<Region>: Region { link: list::Link<()> });

struct Block {
    link: list::Link<()>,
}

intrusive_adapter!(struct BlockAdapter = UnsafeRef<Block>: Block { link: list::Link<()> });
struct BlockHead {
    blocks: list::HList<BlockAdapter>,
    bitmap_offset: u64,
}

static mut REGIONS: list::HList<RegionAdapter> = list::HList::new(RegionAdapter::new());

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

pub unsafe fn add_region(paddr: PAddr, size: u64, flags: RegionFlags) {
    assert!(paddr.is_page_aligned());
    let region = (paddr.as_u64() + virt::PHYSICAL_MAP_BASE.as_u64()) as *mut Region;

    let total_pages = size / PAGE_SIZE64;

    write(
        region,
        Region {
            link: list::Link::UNLINKED,
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
                blocks: list::HList::new(BlockAdapter::new()),
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
            + virt::PHYSICAL_MAP_BASE.as_u64()) as *mut Block;
        write(
            block,
            Block {
                link: list::Link::UNLINKED,
            },
        );
        block_head.blocks.push_front(UnsafeRef::new(block));

        if (order as u8) < biggest_order {
            let page_bit = (current_page >> (order + 1)) + block_head.bitmap_offset;
            let word = page_bit / (size_of::<usize>() as u64);
            let bit = page_bit % (size_of::<usize>() as u64);
            bitmap[word as usize] ^= 1 << bit;
        }

        current_page += 1 << order;
        remaining_pages -= 1 << order;
    }

    REGIONS.push_front(UnsafeRef::new(region));
}

pub unsafe fn add_regions(it: impl IntoIterator<Item = (PAddr, u64, RegionFlags)>) {
    for (paddr, size, flags) in it {
        add_region(paddr, size, flags)
    }
}

unsafe fn alloc_in_region(
    region: &mut Region,
    order: u8,
    flags: AllocFlags,
) -> Result<PAddr, AllocError> {
    if order > region.meta.biggest_order {
        return Err(AllocError);
    }
    let base_paddr = region.base_paddr();
    let meta = region.meta;
    let block_head = &mut region.block_list()[order as usize];
    let bitmap_offset = block_head.bitmap_offset;
    if let Some(block) = block_head.blocks.pop_front() {
        let ptr = block.as_ptr() as *mut Block;
        write_bytes(ptr, 0xcc, 1);

        let paddr = (ptr as u64) - virt::PHYSICAL_MAP_BASE.as_u64();

        if order < meta.biggest_order {
            let bitmap = region.bitmap();
            let page = (paddr - base_paddr.as_u64()) / PAGE_SIZE64 - meta.meta_pages;
            let page_bit = (page >> (order + 1)) + bitmap_offset;
            let word = page_bit / (size_of::<usize>() as u64);
            let bit = page_bit % (size_of::<usize>() as u64);
            bitmap[word as usize] ^= 1 << bit;
        }

        Ok(PAddr::new(paddr))
    } else {
        let block = alloc_in_region(region, order + 1, flags)?;
        let other_block = block.as_u64() + (PAGE_SIZE64 << order);
        put_back_block(region, PAddr::new(other_block), order);
        Ok(block)
    }
}

pub unsafe fn alloc_pages(order: u8, flags: AllocFlags) -> Result<PAddr, AllocError> {
    for region in REGIONS.iter_mut() {
        if region.meta.biggest_order >= order && region.meta.free_pages >= (1 << order) {
            if let Ok(addr) = alloc_in_region(region, order, flags) {
                return Ok(addr);
            }
        }
    }
    Err(AllocError)
}

unsafe fn put_back_block(region: &mut Region, paddr: PAddr, order: u8) {
    let meta = region.meta;
    let base_paddr = region.base_paddr();
    let (blocks, bitmap) = region.block_list_bitmap();
    let block = &mut blocks[order as usize];
    let ptr = (paddr.as_u64() + virt::PHYSICAL_MAP_BASE.as_u64()) as *mut Block;
    write(
        ptr,
        Block {
            link: list::Link::UNLINKED,
        },
    );

    block.blocks.push_front(UnsafeRef::new(ptr));

    let page = (paddr.as_u64() - base_paddr.as_u64()) / PAGE_SIZE64 - meta.meta_pages;
    let page_bit = (page >> (order + 1)) + block.bitmap_offset;
    let word = page_bit / (size_of::<usize>() as u64);
    let bit = page_bit % (size_of::<usize>() as u64);
    bitmap[word as usize] ^= 1 << bit;
}

unsafe fn free_in_region(region: &mut Region, paddr: PAddr, order: u8) {
    let meta = region.meta;
    assert!(order <= meta.biggest_order, "Order is too big");
    let base_paddr = region.base_paddr();
    let (blocks, bitmap) = region.block_list_bitmap();
    let block = &mut blocks[order as usize];
    let page = (paddr.as_u64() - base_paddr.as_u64()) / PAGE_SIZE64 - meta.meta_pages;
    let page_bit = (page >> (order + 1)) + block.bitmap_offset;
    let word = page_bit / (size_of::<usize>() as u64);
    let bit = page_bit % (size_of::<usize>() as u64);
    if order < meta.biggest_order && bitmap[word as usize] & (1 << bit) != 0 {
        let mut cursor = block.blocks.front_mut();
        loop {
            if let Some(b) = cursor.get() {
                let other_paddr = b as *const Block as u64 - virt::PHYSICAL_MAP_BASE.as_u64();
                if paddr.as_u64().abs_diff(other_paddr) == (PAGE_SIZE64 << order) {
                    cursor.unlink();
                    bitmap[word as usize] ^= 1 << bit;
                    let other_paddr = PAddr::new(other_paddr);
                    free_in_region(region, paddr.min(other_paddr), order + 1);
                    break;
                }
            } else {
                panic!(
                    "Could not find block {:012x}",
                    paddr.as_u64() + PAGE_SIZE64 << order
                );
            }
            cursor.move_next();
        }
    } else {
        put_back_block(region, paddr, order);
    }
}

pub unsafe fn dealloc_pages(page: PAddr, order: u8) {
    for region in REGIONS.iter_mut() {
        if region.contains(page) {
            free_in_region(region, page, order);
            return;
        }
    }
    panic!("Invalid address {:?}", page);
}