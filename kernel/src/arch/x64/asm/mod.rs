use core::mem::transmute;

use chos_lib::include_asm;
use chos_lib::mm::VAddr;

include_asm!("./call.S");

extern "C" {
    fn __call_with_stack(
        func: extern "C" fn(u64, u64, u64, u64) -> !,
        stack: VAddr,
        arg0: u64,
        arg1: u64,
        arg2: u64,
        arg3: u64,
    ) -> !;
}

pub unsafe fn call_with_stack0(func: extern "C" fn() -> !, stack: VAddr) -> ! {
    __call_with_stack(transmute(func), stack, 0, 0, 0, 0)
}

pub unsafe fn call_with_stack1(func: extern "C" fn(u64) -> !, stack: VAddr, arg0: u64) -> ! {
    __call_with_stack(transmute(func), stack, arg0, 0, 0, 0)
}

pub unsafe fn call_with_stack2(
    func: extern "C" fn(u64, u64) -> !,
    stack: VAddr,
    arg0: u64,
    arg1: u64,
) -> ! {
    __call_with_stack(transmute(func), stack, arg0, arg1, 0, 0)
}

pub unsafe fn call_with_stack3(
    func: extern "C" fn(u64, u64, u64) -> !,
    stack: VAddr,
    arg0: u64,
    arg1: u64,
    arg2: u64,
) -> ! {
    __call_with_stack(transmute(func), stack, arg0, arg1, arg2, 0)
}

pub unsafe fn call_with_stack4(
    func: extern "C" fn(u64, u64, u64, u64) -> !,
    stack: VAddr,
    arg0: u64,
    arg1: u64,
    arg2: u64,
    arg3: u64,
) -> ! {
    __call_with_stack(transmute(func), stack, arg0, arg1, arg2, arg3)
}
