use core::mem::{align_of, size_of, MaybeUninit};
use core::ptr::{null_mut, write, write_bytes};
use core::slice;

use chos_lib::int::{log2u64, CeilDiv};
use chos_lib::log::debug;

use chos_lib::arch::mm::*;

#[derive(Debug)]
struct Region {
    next: Option<*mut Region>, // This needs to be option since we could be pointing to page 0
    paddr: PAddr,
    total_pages: u64,
    meta_pages: u64,
    free_pages: u64,
    bitmap_size: u64,
    biggest_order: u32,
}

struct BlockHeader {
    next: *mut BlockHeader,
    prev: *mut BlockHeader,
}

type BitmapRepr = usize;

#[derive(Debug)]
struct BlockList {
    blocks: *mut BlockHeader,
    bitmap_offset: usize,
}

impl Region {
    unsafe fn create_region(paddr: PAddr, total_pages: u64, vaddr: VAddr) -> *mut Region {
        // Initialize region header
        let region = vaddr.as_u64() as *mut Region;
        let bitmap_size = total_bitmap_size(total_pages);
        let biggest_order = log2u64(total_pages - 1); // First page is never a data page
        let meta_pages = Self::meta_pages(biggest_order, bitmap_size);
        write(
            region,
            Region {
                next: None,
                paddr,
                total_pages,
                meta_pages,
                free_pages: total_pages - meta_pages,
                bitmap_size,
                biggest_order,
            },
        );

        // Initialize Block Lists
        let mut remaining_bits = bitmap_size;
        let mut bitmap_base = 0;
        for block in Self::block_lists_uninit(region.cast(), biggest_order) {
            *block = MaybeUninit::new(BlockList {
                blocks: null_mut(),
                bitmap_offset: bitmap_base,
            });
            remaining_bits /= 2;
            bitmap_base += remaining_bits as usize;
        }

        // Initialize bitmap
        let bitmap = Self::bitmap_uninit(region.cast(), biggest_order, bitmap_size);
        write_bytes(bitmap.as_mut_ptr(), 0, bitmap.len());
        let bitmap = Self::bitmap(region.cast(), biggest_order, bitmap_size);

        // Save pages
        let mut remaining_pages = (*region).free_pages;
        let mut current_page = 0;
        let block_lists = Self::block_lists(region.cast(), biggest_order);
        while remaining_pages != 0 {
            let order = log2u64(remaining_pages);
            let block_list = &mut block_lists[order as usize];
            debug_assert_eq!(block_list.blocks, null_mut());
            let block_hdr = Self::get_page_ptr(region, current_page).cast::<BlockHeader>();
            write(
                block_hdr,
                BlockHeader {
                    next: null_mut(),
                    prev: null_mut(),
                },
            );
            if order < biggest_order {
                let bit = block_list.bitmap_offset + (current_page >> (order + 1)) as usize;
                let byte = bit / (size_of::<BitmapRepr>() * 8);
                let bit = bit % (size_of::<BitmapRepr>() * 8);
                // debug!("Set bitmap {:02} ({:05} {02})", order, byte, bit);
                bitmap[byte] |= 1 << bit;
            }

            block_list.blocks = block_hdr;
            remaining_pages -= 1 << order;
            current_page += 1 << order;
        }
        // debug!("Region {:#?}", *region);
        // debug!("Blocks {:#?}", Self::block_lists(region.cast(), biggest_order));
        // debug!("Bitmap {:x?}", Self::bitmap(region.cast(), biggest_order, bitmap_size));
        region
    }

    fn block_lists_offset() -> usize {
        let off = size_of::<Self>();
        off.align_up(align_of::<BlockHeader>())
    }

    fn block_lists_len(biggest_order: u32) -> usize {
        biggest_order as usize + 1
    }

    unsafe fn block_lists_uninit<'a>(
        base: *mut u8,
        biggest_order: u32,
    ) -> &'a mut [MaybeUninit<BlockList>] {
        let ptr = base.add(Self::block_lists_offset());
        slice::from_raw_parts_mut(ptr.cast(), Self::block_lists_len(biggest_order))
    }

    unsafe fn block_lists<'a>(base: *mut u8, biggest_order: u32) -> &'a mut [BlockList] {
        MaybeUninit::slice_assume_init_mut(Self::block_lists_uninit(base, biggest_order))
    }

    fn bitmap_offset(biggest_order: u32) -> usize {
        let off = Self::block_lists_offset();
        let off = off + Self::block_lists_len(biggest_order) * size_of::<BlockList>();
        off.align_up(align_of::<BitmapRepr>())
    }

    fn bitmap_len(bitmap_size: u64) -> usize {
        bitmap_size.ceil_div(size_of::<BitmapRepr>() as u64 * 8) as usize
    }

    unsafe fn bitmap_uninit<'a>(
        base: *mut u8,
        biggest_order: u32,
        bitmap_size: u64,
    ) -> &'a mut [MaybeUninit<BitmapRepr>] {
        let ptr = base.add(Self::bitmap_offset(biggest_order));
        slice::from_raw_parts_mut(ptr.cast(), Self::bitmap_len(bitmap_size))
    }

    unsafe fn bitmap<'a>(
        base: *mut u8,
        biggest_order: u32,
        bitmap_size: u64,
    ) -> &'a mut [BitmapRepr] {
        MaybeUninit::slice_assume_init_mut(Self::bitmap_uninit(base, biggest_order, bitmap_size))
    }

    fn meta_pages(biggest_order: u32, bitmap_size: u64) -> u64 {
        let off = Self::bitmap_offset(biggest_order);
        let off = off + Self::bitmap_len(bitmap_size) * size_of::<BitmapRepr>();
        (off as u64).ceil_div(PAGE_SIZE64)
    }

    unsafe fn get_page_ptr(region: *mut Self, n: u64) -> *mut u8 {
        region
            .cast::<u8>()
            .add(((n + (*region).meta_pages) * PAGE_SIZE64) as usize)
    }

    unsafe fn contains(&self, addr: PAddr) -> bool {
        let start = (self as *const Self) as u64;
        let end = start + self.total_pages * PAGE_SIZE64;
        addr.as_u64() >= start && addr.as_u64() < end
    }

    unsafe fn allocate(&mut self, order: u32) -> Option<PAddr> {
        if order > self.biggest_order {
            return None;
        }
        let base = (self as *mut Self).cast();
        let block_lists = Self::block_lists(base, self.biggest_order);
        let block_list = &mut block_lists[order as usize];
        let paddr = if block_list.blocks == null_mut() {
            self.free_pages += 1 << order;
            let block = self.allocate(order + 1)?;
            let other = block.add(PAGE_SIZE64 << order);
            let other = other.as_u64() as *mut BlockHeader;
            write(
                other,
                BlockHeader {
                    next: block_list.blocks,
                    prev: null_mut(),
                },
            );
            if (*other).next != null_mut() {
                (*(*other).next).prev = other;
            }
            block_list.blocks = other;
            block
        } else {
            self.free_pages -= 1 << order;
            let block = block_list.blocks;
            block_list.blocks = (*block).next;
            if block_list.blocks != null_mut() {
                (*block_list.blocks).prev = null_mut();
            }
            write_bytes(block, 0xcc, 1);
            PAddr::new(block as u64)
        };
        let page = (paddr.as_u64() - base as u64) / PAGE_SIZE64 - self.meta_pages;

        if order < self.biggest_order {
            let bitmap = Self::bitmap(base, self.biggest_order, self.bitmap_size);
            let bit = block_list.bitmap_offset + (page >> (order + 1)) as usize;
            debug_assert!(bit < (self.bitmap_size as usize));
            let word = bit / (size_of::<BitmapRepr>() * 8);
            let bit = bit % (size_of::<BitmapRepr>() * 8);
            bitmap[word] ^= 1 << bit;
        }
        Some(paddr)
    }

    unsafe fn merge_blocks(
        &mut self,
        block_lists: &mut [BlockList],
        bitmap: &mut [BitmapRepr],
        page: u64,
        order: u32,
    ) {
        assert!(order < self.biggest_order);
        let base = self as *mut Self as u64;
        let block_list = &mut block_lists[order as usize];

        let mut other = None;
        let mut cur = block_list.blocks;
        while cur != null_mut() {
            let other_page = (cur as u64 - base) / PAGE_SIZE64 - self.meta_pages;
            assert_ne!(page, other_page);
            if (page >> (order + 1)) == (other_page >> (order + 1)) {
                other = Some(cur);
                break;
            }
            cur = (*cur).next;
        }
        let other = other.expect("The block was not found in the list");
        if (*other).next != null_mut() {
            (*(*other).next).prev = (*other).prev;
        }
        if (*other).prev != null_mut() {
            (*(*other).prev).next = (*other).next;
        }
        if block_list.blocks == other {
            block_list.blocks = (*other).next;
        }
        write_bytes(other, 0xcc, 1);

        let page = page & !((1 << (order + 1)) - 1);
        self.deallocate_inner(block_lists, bitmap, page, order + 1);
    }

    unsafe fn deallocate_inner(
        &mut self,
        block_lists: &mut [BlockList],
        bitmap: &mut [BitmapRepr],
        page: u64,
        order: u32,
    ) {
        let base = self as *mut Self as u64;

        let mut merged: bool = false;
        if order < self.biggest_order {
            let block_list = &mut block_lists[order as usize];
            let bit = block_list.bitmap_offset + (page >> (order + 1)) as usize;
            debug_assert!(bit < (self.bitmap_size as usize));
            let word = bit / (size_of::<BitmapRepr>() * 8);
            let bit = bit % (size_of::<BitmapRepr>() * 8);
            bitmap[word] ^= 1 << bit;
            if (bitmap[word] & (1 << bit)) == 0 {
                self.merge_blocks(block_lists, bitmap, page, order);
                merged = true;
                self.free_pages -= 1 << order;
            }
        }
        if !merged {
            let block_list = &mut block_lists[order as usize];
            let block = (base + (page + self.meta_pages) * PAGE_SIZE64) as *mut BlockHeader;
            write(
                block,
                BlockHeader {
                    next: null_mut(),
                    prev: null_mut(),
                },
            );
            (*block).next = block_list.blocks;
            if (*block).next != null_mut() {
                (*(*block).next).prev = block;
            }
            block_list.blocks = block;
            self.free_pages += 1 << order;
        }
    }

    unsafe fn deallocate(&mut self, paddr: PAddr, order: u32) {
        debug_assert!(order <= self.biggest_order);
        let base = (self as *mut Self).cast::<u8>();
        let page = (paddr.as_u64() - base as u64) / PAGE_SIZE64 - self.meta_pages;
        let bitmap = Self::bitmap(base, self.biggest_order, self.bitmap_size);
        let block_lists = Self::block_lists(base, self.biggest_order);
        self.deallocate_inner(block_lists, bitmap, page, order)
    }

    unsafe fn remap(&mut self, offset: isize) {
        let block_lists = Self::block_lists((self as *mut Self).cast(), self.biggest_order);
        for block_list in block_lists {
            let mut cur = block_list.blocks;
            while cur != null_mut() {
                let next = (*cur).next;
                (*cur).next = (*cur).next.cast::<u8>().offset(offset).cast();
                (*cur).prev = (*cur).prev.cast::<u8>().offset(offset).cast();
                cur = next;
            }
            block_list.blocks = block_list.blocks.cast::<u8>().offset(offset).cast();
        }
    }
}
static mut REGIONS: Option<*mut Region> = None;

fn total_bitmap_size(pages: u64) -> u64 {
    if pages > 1 {
        pages / 2 + total_bitmap_size(pages / 2)
    } else {
        0
    }
}

pub unsafe fn add_region(paddr: PAddr, size: u64, vaddr: VAddr) {
    assert_eq!(
        size % PAGE_SIZE64,
        0,
        "Size must be a multiple of page size"
    );
    let region = Region::create_region(paddr, size / PAGE_SIZE64, vaddr);
    (*region).next = REGIONS;
    REGIONS = Some(region);
}

pub unsafe fn add_regions(it: impl IntoIterator<Item = (PAddr, u64, VAddr)>) {
    for (paddr, size, vaddr) in it {
        add_region(paddr, size, vaddr)
    }
}

pub unsafe fn remap_regions(mut f: impl FnMut(*mut (), u64) -> *mut ()) {
    let mut cur = &mut REGIONS;
    while let Some(region) = cur {
        let old_addr = region.cast();
        let new_addr = f(old_addr, (**region).total_pages * PAGE_SIZE64);
        let offset = new_addr.cast::<u8>().offset_from(old_addr.cast());
        debug_assert_eq!(offset % PAGE_SIZE as isize, 0, "");
        *region = new_addr.cast();
        (**region).remap(offset);
        cur = &mut (**region).next;
    }
}

unsafe fn find_map_regions<R, F: FnMut(&mut Region) -> Option<R>>(mut f: F) -> Option<R> {
    let mut cur = REGIONS;
    while let Some(region) = cur {
        let region = &mut *region;
        if let Some(res) = f(region) {
            return Some(res);
        }
        cur = region.next;
    }
    None
}

unsafe fn debug_alloc(msg: &str) {
    debug!("{}\n{:#?}", msg, (*REGIONS.unwrap_unchecked()));
    let block_lists = Region::block_lists(
        REGIONS.unwrap_unchecked().cast(),
        (*REGIONS.unwrap_unchecked()).biggest_order,
    );
    for (i, b) in block_lists.iter().enumerate() {
        let mut cur = b.blocks;
        debug!("  [{:02}]", i);
        while cur != null_mut() {
            debug!("    {:p}", cur);
            cur = (*cur).next;
        }
    }
}

pub unsafe fn allocate_pages(order: u32) -> Option<PAddr> {
    let res = find_map_regions(|region| {
        if order <= region.biggest_order {
            if let Some(addr) = region.allocate(order) {
                return Some(addr);
            }
        }
        None
    });
    res
}

pub unsafe fn deallocate_pages(page: PAddr, order: u32) {
    assert!(page.is_page_aligned());
    find_map_regions(|region| {
        region
            .contains(page)
            .then(|| region.deallocate(page, order))
    });
}

pub fn free_pages() -> u64 {
    let mut pages = 0;
    unsafe {
        find_map_regions::<(), _>(|region| {
            pages += region.free_pages;
            None
        })
    };
    pages
}
