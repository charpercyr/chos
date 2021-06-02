#![no_std]

#![feature(allocator_api)]
#![feature(asm)]
#![feature(decl_macro)]
#![feature(maybe_uninit_ref)]

mod arch;

#[panic_handler]
fn panic(_: &core::panic::PanicInfo) -> ! {
    loop {}
}

#[no_mangle]
pub extern "C" fn entry() -> ! {
    loop {}
}
