#![no_std]

#![feature(allocator_api)]
#![feature(asm)]
#![feature(extended_key_value_attributes)]
#![feature(maybe_uninit_ref)]

#[macro_use]
mod arch;

#[panic_handler]
fn panic(_: &core::panic::PanicInfo) -> ! {
    loop {}
}

#[no_mangle]
pub extern "C" fn entry() -> ! {
    loop {}
}
