use core::alloc::{AllocError, Layout};
use core::marker::PhantomData;
use core::mem::{align_of, size_of};
use core::ptr::{read, write, write_bytes, NonNull};
use core::slice::from_raw_parts_mut;

use chos_lib::arch::mm::VAddr;
use chos_lib::init::ConstInit;
use chos_lib::int::{CeilDiv, align_upusize, ceil_divusize};
use chos_lib::intrusive::{self as int, list, UnsafeRef};
use chos_lib::bitmap::{self, Bitmap};

pub trait Slab: Sized {
    const SIZE: usize;

    fn frame_containing(addr: VAddr) -> Self;
    fn vaddr(&self) -> VAddr;
}

pub unsafe trait SlabAllocator {
    type Slab: Slab;

    unsafe fn alloc_slab(&mut self) -> Result<Self::Slab, AllocError>;
    unsafe fn dealloc_slab(&mut self, frame: Self::Slab);
}

struct SlabMeta {
    layout: Layout,
    object_count: usize,
}

struct SlabHeader<F: SlabAllocator> {
    link: list::Link<()>,
    frame: F::Slab,
}

impl<F: SlabAllocator> SlabHeader<F> {
    unsafe fn alloc(&mut self, meta: &SlabMeta) -> Result<NonNull<[u8]>, AllocError> {
        let bitmap = self.bitmap_mut(meta);
        let i = bitmap.leading_ones() as usize;
        if i < meta.object_count {
            bitmap.set_bit(i, true);
            Ok(self.get_object_ptr(meta, i))
        } else {
            return Err(AllocError);
        }
    }

    unsafe fn dealloc(&mut self, ptr: NonNull<[u8]>, meta: &SlabMeta) {
        chos_lib::ptr::write_bytes_slice(ptr.as_ptr(), 0xcc);
        let first_object = self.get_object_ptr(meta, 0);
        let bitmap = self.bitmap_mut(meta);
        let idx = (ptr
            .cast::<u8>()
            .as_ptr()
            .offset_from(first_object.cast().as_ptr()) as usize)
            / meta.layout.size();
        bitmap.set_bit(idx, true);
    }

    fn is_empty(&self) -> bool {
        false
    }

    fn is_full(&self) -> bool {
        false
    }

    unsafe fn bitmap_mut(&mut self, meta: &SlabMeta) -> &mut Bitmap {
        let ptr = self as *mut Self as *mut u8;
        let ptr = ptr.add(size_of::<Self>());
        let ptr = ptr.add(ptr.align_offset(align_of::<usize>()));
        Bitmap::from_raw_parts_mut(ptr.cast(), meta.object_count.ceil_div(size_of::<usize>()))
    }

    unsafe fn get_object_ptr(&self, meta: &SlabMeta, i: usize) -> NonNull<[u8]> {
        let ptr = self as *const Self as *mut u8;
        let ptr = ptr.add(size_of::<Self>());
        let ptr = ptr.add(ptr.align_offset(align_of::<usize>()));
        let ptr = ptr.add(size_of::<usize>() * meta.object_count.ceil_div(size_of::<usize>()));
        let ptr = ptr.add(ptr.align_offset(meta.layout.align()));
        let ptr = ptr.add(i * meta.layout.size());
        from_raw_parts_mut(ptr, meta.layout.size()).into()
    }
}

struct SlabAdapter<F: SlabAllocator>(PhantomData<F>);
impl<F: SlabAllocator> int::Adapter for SlabAdapter<F> {
    type Value = SlabHeader<F>;
    type Link = list::Link<()>;
    type Pointer = UnsafeRef<SlabHeader<F>>;

    unsafe fn get_link(&self, value: *const Self::Value) -> *const Self::Link {
        &(*value).link
    }
    unsafe fn get_value(&self, link: *const Self::Link) -> *const Self::Value {
        chos_lib::container_of!(link, link, SlabHeader<F>)
    }
}
impl<F: SlabAllocator> ConstInit for SlabAdapter<F> {
    const INIT: Self = Self(PhantomData);
}

pub struct RawSlabAllocator<F: SlabAllocator> {
    frame_alloc: F,
    meta: SlabMeta,
    empty: list::HList<SlabAdapter<F>>,
    partial: list::HList<SlabAdapter<F>>,
    full: list::HList<SlabAdapter<F>>,
}

impl<F: SlabAllocator> RawSlabAllocator<F> {
    pub const fn new(frame_alloc: F, layout: Layout) -> Self {
        assert!(2 * <F::Slab as Slab>::SIZE > 3 * layout.size());
        Self {
            frame_alloc,
            meta: SlabMeta {
                layout,
                object_count: estimate_bitmap_bits::<F>(layout),
            },
            empty: list::HList::INIT,
            partial: list::HList::INIT,
            full: list::HList::INIT,
        }
    }

    pub unsafe fn alloc(&mut self) -> Result<NonNull<[u8]>, AllocError> {
        // Try to allocate in partial slabs
        if let Some(uref) = self.partial.pop_front() {
            let slab = &mut *(uref.as_ptr() as *mut SlabHeader<F>);
            if let Ok(ptr) = slab.alloc(&self.meta) {
                if slab.is_full() {
                    self.full.push_front(uref);
                } else {
                    self.partial.push_front(uref);
                }
                return Ok(ptr);
            }
        }
        // Try to allocate in empty slabs
        if let Some(uref) = self.empty.pop_front() {
            let slab = &mut *(uref.as_ptr() as *mut SlabHeader<F>);
            if let Ok(ptr) = slab.alloc(&self.meta) {
                if slab.is_full() {
                    self.full.push_front(uref);
                } else {
                    self.partial.push_front(uref);
                }
                return Ok(ptr);
            }
        }
        // Try to allocate in a new slab
        let mut new_slab_ptr = self.alloc_new_slab()?;
        let new_slab = new_slab_ptr.as_mut();
        let ptr = new_slab.alloc(&self.meta).map_err(|e| {
            self.dealloc_slab(new_slab_ptr);
            e
        })?;
        let uref = UnsafeRef::new(new_slab);
        if new_slab.is_full() {
            self.full.push_front(uref);
        } else {
            self.partial.push_front(uref);
        }
        Ok(ptr)
    }

    pub unsafe fn dealloc(&mut self, ptr: NonNull<[u8]>) {
        assert_eq!(ptr.as_ref().len(), self.meta.layout.size());
        let vaddr = VAddr::new_unchecked(ptr.as_ptr() as *mut u8 as u64);
        let vaddr = <F::Slab as Slab>::frame_containing(vaddr).vaddr();
        let slab: &mut SlabHeader<F> = &mut *vaddr.as_ptr_mut();
        let was_full = slab.is_full();
        slab.dealloc(ptr, &self.meta);
        if was_full {
            let uref = self
                .full
                .cursor_mut_from_pointer(slab)
                .unlink()
                .expect("Should be a valid cursor");
            if slab.is_empty() {
                self.empty.push_front(uref);
            } else {
                self.partial.push_front(uref);
            }
        } else {
            if slab.is_empty() {
                let uref = self
                    .partial
                    .cursor_mut_from_pointer(slab)
                    .unlink()
                    .expect("Should be a valid cursor");
                self.empty.push_front(uref);
            }
        }
    }

    pub unsafe fn dealloc_empty_frames(&mut self) {
        while let Some(slab) = self.empty.pop_front() {
            self.dealloc_slab(NonNull::new_unchecked(slab.as_ptr() as *mut _));
        }
    }

    unsafe fn alloc_new_slab(&mut self) -> Result<NonNull<SlabHeader<F>>, AllocError> {
        let frame = self.frame_alloc.alloc_slab()?;
        let ptr: *mut SlabHeader<F> = frame.vaddr().as_ptr_mut();
        write_bytes(ptr.cast::<u8>(), 0, <F::Slab as Slab>::SIZE);
        write(
            ptr,
            SlabHeader {
                frame,
                link: list::Link::UNLINKED,
            },
        );
        let slab = &mut *ptr;
        slab.bitmap_mut(&self.meta).set_all_in(..self.meta.object_count);
        Ok(NonNull::new_unchecked(ptr))
    }

    unsafe fn dealloc_slab(&mut self, slab: NonNull<SlabHeader<F>>) {
        let slab = read(slab.as_ptr());
        self.frame_alloc.dealloc_slab(slab.frame);
    }
}

pub const fn estimate_bitmap_bits<F: SlabAllocator>(layout: Layout) -> usize {
    let header_bytes = align_upusize(size_of::<SlabHeader<F>>(), align_of::<usize>());
    let object_bytes = align_upusize(layout.size(), layout.align());
    let mut object_count = ceil_divusize(<F::Slab as Slab>::SIZE - header_bytes, object_bytes + 1);
    // We might overestimate
    while header_bytes + align_upusize(ceil_divusize(object_count, bitmap::REPR_BITS), layout.align()) + object_bytes * object_count > <F::Slab as Slab>::SIZE {
        object_count -= 1;
    }
    object_count
}
