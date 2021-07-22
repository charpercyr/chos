
pub mod alloc;

use chos_lib::pool::{IArc, IArcAdapter, IArcCount, Pool};

use core::alloc::AllocError;
use core::ptr::NonNull;

use super::{PAddr, VAddr};

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

struct PagePoolImpl;
chos_lib::pool!(PagePool: Page => &PagePoolImpl);

unsafe impl Pool<Page> for PagePoolImpl {
    unsafe fn allocate(&self) -> Result<NonNull<Page>, AllocError> {
        Err(AllocError)
    }

    unsafe fn deallocate(&self, ptr: NonNull<Page>) {
        
    }
}

pub type PagePtr = IArc<Page, PagePool>;

pub unsafe fn allocate_pages(order: u32) -> Result<PagePtr, AllocError> {
    Err(AllocError)
}
