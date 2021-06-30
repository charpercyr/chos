#![no_std]

#![feature(allocator_api)]
#![feature(asm)]
#![feature(decl_macro)]
#![feature(thread_local)]

mod arch;
mod percpu;

#[panic_handler]
fn panic(_: &core::panic::PanicInfo) -> ! {
    loop {}
}

percpu! {
    pub static mut ref FOO: usize = 0;
    pub static mut ref BAR: usize = 1;
    pub static mut ref BAZ: [usize; 4] = [0, 1, 2, 3];
}

#[no_mangle]
pub extern "C" fn entry() -> ! {
    loop {}
}
