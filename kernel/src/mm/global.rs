use alloc::alloc::{GlobalAlloc, Layout};

struct KAlloc;

unsafe impl GlobalAlloc for KAlloc {
    unsafe fn alloc(&self, _layout: Layout) -> *mut u8 {
        core::ptr::null_mut()
    }
    unsafe fn dealloc(&self, _ptr: *mut u8, _layout: Layout) {}
}

#[global_allocator]
static KALLOC: KAlloc = KAlloc;

#[no_mangle]
fn rust_oom(layout: Layout) -> ! {
    panic!(
        "Out of memory, tried to allocate {} bytes (align = {})",
        layout.size(),
        layout.align()
    );
}
