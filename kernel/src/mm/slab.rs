use core::alloc::{AllocError, Layout};
use core::marker::PhantomData;
use core::mem::{align_of, size_of};
use core::ptr::{read, write, write_bytes, NonNull};
use core::slice::from_raw_parts_mut;

use bitvec::slice::BitSlice;
use chos_lib::mm::VAddr;
use chos_lib::init::ConstInit;
use chos_lib::int::{align_upusize, ceil_divusize};
use chos_lib::pool::Pool;
use chos_lib::sync::lock::{Lock, RawLock};
use chos_lib::sync::spin::lock::RawSpinLock;
use intrusive_collections::{
    intrusive_adapter, linked_list, LinkedList, LinkedListAtomicLink, UnsafeMut,
};

use super::phys::MMSlabAllocator;

pub trait Slab: Sized {
    const SIZE: usize;

    fn frame_containing(addr: VAddr) -> VAddr;
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
    link: LinkedListAtomicLink,
    frame: F::Slab,
}

impl<F: SlabAllocator> SlabHeader<F> {
    unsafe fn alloc(&mut self, meta: &SlabMeta) -> Result<NonNull<[u8]>, AllocError> {
        let bitmap = self.bitmap_mut(meta);
        let i = bitmap.leading_zeros();
        if i < meta.object_count {
            bitmap.set(i, false);
            Ok(self.get_object_ptr(meta, i))
        } else {
            Err(AllocError)
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
        bitmap.set(idx, true);
    }

    fn is_empty(&self, meta: &SlabMeta) -> bool {
        let bitmap = unsafe { self.bitmap(meta) };
        bitmap.all()
    }

    fn is_full(&self, meta: &SlabMeta) -> bool {
        let bitmap = unsafe { self.bitmap(meta) };
        bitmap.not_any()
    }

    unsafe fn bitmap(&self, meta: &SlabMeta) -> &BitSlice {
        use bitvec::prelude::*;
        let ptr = self as *const Self as *const u8;
        let ptr = ptr.add(Self::bitmap_offset());
        bitvec::slice::from_raw_parts(BitPtr::from_ptr(ptr.cast()).unwrap(), meta.object_count)
            .unwrap()
    }

    unsafe fn bitmap_mut(&mut self, meta: &SlabMeta) -> &mut BitSlice {
        use bitvec::prelude::*;
        let ptr = self as *mut Self as *mut u8;
        let ptr = ptr.add(Self::bitmap_offset());
        bitvec::slice::from_raw_parts_mut(
            BitPtr::from_mut_ptr(ptr.cast()).unwrap(),
            meta.object_count,
        )
        .unwrap()
    }

    unsafe fn get_object_ptr(&self, meta: &SlabMeta, i: usize) -> NonNull<[u8]> {
        let ptr = self as *const Self as *mut u8;
        let ptr = ptr.add(Self::object_offset(meta, i));
        from_raw_parts_mut(ptr, meta.layout.size()).into()
    }

    const fn bitmap_offset() -> usize {
        let off = size_of::<Self>();
        align_upusize(off, align_of::<usize>())
    }

    const fn object_offset(meta: &SlabMeta, i: usize) -> usize {
        let off = Self::bitmap_offset();
        let off =
            off + size_of::<usize>() * ceil_divusize(meta.object_count, size_of::<usize>() * 8);
        let off = align_upusize(off, align_of::<usize>());
        off + i * meta.layout.size()
    }
}

intrusive_collections::intrusive_adapter!(SlabAdapter<F> = UnsafeMut<SlabHeader<F>> : SlabHeader<F> { link: linked_list::AtomicLink } where F: SlabAllocator);

#[derive(Debug, Clone, Copy)]
pub struct ObjectAllocatorStats {
    pub empty_slabs: usize,
    pub partial_slabs: usize,
    pub full_slabs: usize,
    pub free_objects: usize,
    pub allocated_objects: usize,
}

pub struct RawObjectAllocator<F: SlabAllocator> {
    frame_alloc: F,
    meta: SlabMeta,
    empty: LinkedList<SlabAdapter<F>>,
    partial: LinkedList<SlabAdapter<F>>,
    full: LinkedList<SlabAdapter<F>>,
    stats: ObjectAllocatorStats,
}

impl<F: SlabAllocator> RawObjectAllocator<F> {
    pub const fn new(frame_alloc: F, layout: Layout) -> Self {
        assert!(2 * <F::Slab as Slab>::SIZE > 3 * layout.size());
        Self {
            frame_alloc,
            meta: slab_meta::<F>(layout),
            empty: LinkedList::new(SlabAdapter::NEW),
            partial: LinkedList::new(SlabAdapter::NEW),
            full: LinkedList::new(SlabAdapter::NEW),
            stats: ObjectAllocatorStats {
                empty_slabs: 0,
                partial_slabs: 0,
                full_slabs: 0,
                free_objects: 0,
                allocated_objects: 0,
            },
        }
    }

    pub unsafe fn alloc(&mut self) -> Result<NonNull<[u8]>, AllocError> {
        // Try to allocate in partial slabs
        if let Some(mut uref) = self.partial.pop_front() {
            let slab = &mut *(uref.as_mut() as *mut SlabHeader<F>);
            if let Ok(ptr) = slab.alloc(&self.meta) {
                if slab.is_full(&self.meta) {
                    self.stats.partial_slabs -= 1;
                    self.stats.full_slabs += 1;
                    self.full.push_front(uref);
                } else {
                    self.partial.push_front(uref);
                }
                self.stats.free_objects -= 1;
                self.stats.allocated_objects += 1;
                return Ok(ptr);
            }
        }
        // Try to allocate in empty slabs
        if let Some(mut uref) = self.empty.pop_front() {
            let slab = &mut *(uref.as_mut() as *mut SlabHeader<F>);
            if let Ok(ptr) = slab.alloc(&self.meta) {
                self.stats.empty_slabs -= 1;
                if slab.is_full(&self.meta) {
                    self.stats.full_slabs += 1;
                    self.full.push_front(uref);
                } else {
                    self.stats.partial_slabs += 1;
                    self.partial.push_front(uref);
                }
                self.stats.free_objects -= 1;
                self.stats.allocated_objects += 1;
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
        let uref = UnsafeMut::from_raw(new_slab);
        if new_slab.is_full(&self.meta) {
            self.stats.full_slabs += 1;
            self.full.push_front(uref);
        } else {
            self.stats.partial_slabs += 1;
            self.partial.push_front(uref);
        }
        self.stats.free_objects += self.meta.object_count - 1;
        self.stats.allocated_objects += 1;
        Ok(ptr)
    }

    pub unsafe fn dealloc(&mut self, ptr: NonNull<[u8]>) {
        assert_eq!(ptr.as_ref().len(), self.meta.layout.size());
        let vaddr = VAddr::new_unchecked(ptr.as_ptr() as *mut u8 as u64);
        let vaddr = <F::Slab as Slab>::frame_containing(vaddr);
        let slab: &mut SlabHeader<F> = &mut *vaddr.as_mut_ptr();
        let was_full = slab.is_full(&self.meta);
        slab.dealloc(ptr, &self.meta);
        if was_full {
            let uref = self.full.cursor_mut_from_ptr(slab).remove().unwrap();
            self.stats.full_slabs -= 1;
            if slab.is_empty(&self.meta) {
                self.stats.empty_slabs += 1;
                self.empty.push_front(uref);
            } else {
                self.stats.partial_slabs += 1;
                self.partial.push_front(uref);
            }
        } else if slab.is_empty(&self.meta) {
            self.stats.partial_slabs -= 1;
            self.stats.empty_slabs += 1;
            let uref = self.partial.cursor_mut_from_ptr(slab).remove().unwrap();
            self.empty.push_front(uref);
        }
        self.stats.allocated_objects -= 1;
        self.stats.free_objects += 1;
    }

    pub unsafe fn dealloc_empty_frames(&mut self) {
        while let Some(mut slab) = self.empty.pop_front() {
            self.stats.free_objects -= self.meta.object_count;
            self.dealloc_slab(NonNull::new_unchecked(slab.as_mut() as *mut _));
        }
        self.stats.empty_slabs = 0;
    }

    pub fn stats(&self) -> &ObjectAllocatorStats {
        &self.stats
    }

    pub fn layout(&self) -> Layout {
        self.meta.layout
    }

    unsafe fn alloc_new_slab(&mut self) -> Result<NonNull<SlabHeader<F>>, AllocError> {
        let frame = self.frame_alloc.alloc_slab()?;
        let ptr: *mut SlabHeader<F> = frame.vaddr().as_mut_ptr();
        write_bytes(ptr.cast::<u8>(), 0, <F::Slab as Slab>::SIZE);
        write(
            ptr,
            SlabHeader {
                frame,
                link: LinkedListAtomicLink::new(),
            },
        );
        let slab = &mut *ptr;
        slab.bitmap_mut(&self.meta).set_all(true);
        Ok(NonNull::new_unchecked(ptr))
    }

    unsafe fn dealloc_slab(&mut self, slab: NonNull<SlabHeader<F>>) {
        let slab = read(slab.as_ptr());
        self.frame_alloc.dealloc_slab(slab.frame);
    }
}

pub struct ObjectAllocator<F: SlabAllocator, T> {
    raw: RawObjectAllocator<F>,
    phantom: PhantomData<T>,
}

impl<T, F: SlabAllocator> ObjectAllocator<F, T> {
    pub const fn new(frame_alloc: F) -> Self {
        Self {
            raw: RawObjectAllocator::new(frame_alloc, Layout::new::<T>()),
            phantom: PhantomData,
        }
    }

    pub unsafe fn alloc(&mut self) -> Result<NonNull<T>, AllocError> {
        self.raw.alloc().map(|ptr| ptr.cast())
    }

    pub unsafe fn dealloc(&mut self, ptr: NonNull<T>) {
        self.raw
            .dealloc(NonNull::from_raw_parts(ptr.cast(), size_of::<T>()))
    }

    pub fn stats(&self) -> &ObjectAllocatorStats {
        self.raw.stats()
    }
}

impl<T, F: SlabAllocator + ConstInit> ConstInit for ObjectAllocator<F, T> {
    const INIT: Self = Self::new(ConstInit::INIT);
}

const fn slab_meta<F: SlabAllocator>(layout: Layout) -> SlabMeta {
    let header_bytes = align_upusize(size_of::<SlabHeader<F>>(), align_of::<usize>());
    let object_bytes = align_upusize(layout.size(), layout.align());
    let mut meta = SlabMeta {
        layout,
        object_count: (<F::Slab as Slab>::SIZE - header_bytes) / object_bytes,
    };
    // We might overestimate
    while SlabHeader::<F>::object_offset(&meta, meta.object_count) > <F::Slab as Slab>::SIZE {
        meta.object_count -= 1;
    }
    meta
}

pub struct PoolObjectAllocator<L: RawLock, F: SlabAllocator, T> {
    alloc: Lock<L, ObjectAllocator<F, T>>,
}

impl<L: RawLock, F: SlabAllocator, T> PoolObjectAllocator<L, F, T> {
    pub const fn new(frame_alloc: F) -> Self
    where
        L: ConstInit,
    {
        Self::new_with_lock(frame_alloc, L::INIT)
    }
    pub const fn new_with_lock(frame_alloc: F, lock: L) -> Self {
        Self {
            alloc: Lock::new_with(ObjectAllocator::new(frame_alloc), lock),
        }
    }
}

impl<L: RawLock + ConstInit, F: SlabAllocator + ConstInit, T> ConstInit
    for PoolObjectAllocator<L, F, T>
{
    const INIT: Self = Self::new(ConstInit::INIT);
}

unsafe impl<L: RawLock, F: SlabAllocator, T> Pool<T> for PoolObjectAllocator<L, F, T> {
    unsafe fn allocate(&self) -> Result<NonNull<T>, AllocError> {
        self.alloc.lock().alloc()
    }
    unsafe fn deallocate(&self, ptr: NonNull<T>) {
        self.alloc.lock().dealloc(ptr)
    }
}

pub type DefaultPoolObjectAllocator<T, const O: u8> =
    PoolObjectAllocator<RawSpinLock, MMSlabAllocator<O>, T>;

pub macro object_pool {
    (struct $name:ident (order = $order:expr) : $typ:ty) => {
        paste::item! {
            static [<__ $name:snake:upper _IMPL>]: $crate::mm::slab::DefaultPoolObjectAllocator<$typ, $order> =
                chos_lib::init::ConstInit::INIT;
            chos_lib::pool!(struct $name: $typ => &[<__ $name:snake:upper _IMPL>]);
        }
    },
    (struct $name:ident : $typ:ty) => {
        $crate::mm::slab::object_pool!(struct $name (order = 0) : $typ);
    },
}
