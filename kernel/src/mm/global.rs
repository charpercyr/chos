
use alloc::alloc::GlobalAlloc;

struct KAlloc;

unsafe impl GlobalAlloc for KAlloc {
    unsafe fn alloc(&self, _layout: core::alloc::Layout) -> *mut u8 {
        core::ptr::null_mut()
    }
    unsafe fn dealloc(&self, _ptr: *mut u8, _layout: core::alloc::Layout) {
    }
}

#[global_allocator]
static KALLOC: KAlloc = KAlloc;

#[no_mangle]
fn rust_oom() -> ! {
    panic!("Out of memory");
}
