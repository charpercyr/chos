pub mod alloc;

use super::{PAddr, VAddr};

use core::alloc::AllocError;
use core::ptr::NonNull;

use chos_lib::pool::{IArc, IArcAdapter, IArcCount, Pool};

pub struct Page {
    refcount: IArcCount,
    order: u32,
    vaddr: VAddr,
    paddr: PAddr,
}
impl IArcAdapter for Page {
    fn count(&self) -> &IArcCount {
        &self.refcount
    }
}

pub type PagePtr = IArc<Page, PageAlloc>;

pub struct PageAlloc;

unsafe impl Pool<Page> for PageAlloc {
    unsafe fn allocate(&self) -> Result<NonNull<Page>, AllocError> {
        todo!()
    }

    unsafe fn deallocate(&self, _: NonNull<Page>) {
        todo!()
    }
}

chos_lib::pool!(PageAdapter: Page => &PageAlloc);

pub unsafe fn allocate_pages(_: u32) -> PagePtr {
    todo!()
}
