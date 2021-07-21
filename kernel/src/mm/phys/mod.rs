
pub mod alloc;

use chos_lib::pool::{IArc, IArcAdapter, IArcCount, Pool};

use core::alloc::AllocError;
use core::ptr::NonNull;

pub struct Page {
    refcount: IArcCount,
}

impl IArcAdapter for Page {
    fn count(&self) -> &IArcCount {
        &self.refcount
    }
}

struct PageStructPoolImpl;

unsafe impl Pool<Page> for PageStructPoolImpl {
    unsafe fn allocate(&self) -> Result<NonNull<Page>, AllocError> {
        Err(AllocError)
    }

    unsafe fn deallocate(&self, ptr: NonNull<Page>) {
        
    }
}

chos_lib::pool!(PageStructPool: Page => &PageStructPoolImpl);
