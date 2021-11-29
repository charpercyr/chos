use alloc::alloc::{GlobalAlloc, Layout};

struct KAlloc;

unsafe impl GlobalAlloc for KAlloc {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        unimplemented!("KAlloc::alloc({:?})", layout)
    }
    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        unimplemented!("KAlloc::dealloc({:p}, {:?}", ptr, layout)
    }
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
