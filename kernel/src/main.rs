#![no_std]
#![no_main]

#![feature(asm)]

#[panic_handler]
fn panic(_: &core::panic::PanicInfo) -> ! {
    loop {}
}

#[no_mangle]
pub static mut VAR: usize = 0;

#[no_mangle]
pub extern "C" fn entry() -> ! {
    unsafe {
        panic!("{}", VAR);
    }
}