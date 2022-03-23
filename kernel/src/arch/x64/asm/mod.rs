use chos_lib::include_asm;
use chos_lib::mm::VAddr;

include_asm!("./call.S");

extern "C" {
    pub fn call_with_stack(
        func: extern "C" fn(u64, u64, u64, u64) -> !,
        stack: VAddr,
        arg0: u64,
        arg1: u64,
        arg2: u64,
        arg3: u64,
    ) -> !;
}
