use core::arch::asm;

use crate::arch::mm::VAddr;
use crate::arch::msr::Msr;

#[derive(Debug)]
pub struct CS;

impl CS {
    pub fn get() -> u16 {
        unsafe {
            let value;
            asm! {
                "mov %cs, {:x}",
                out(reg) value,
                options(att_syntax, nomem, nostack),
            }
            value
        }
    }
}

const FSBASE: Msr = Msr::new(0xc0000100);
pub struct FS;
impl FS {
    pub fn get() -> VAddr {
        unsafe { VAddr::new_unchecked(FSBASE.read()) }
    }
    pub unsafe fn set(addr: VAddr) {
        FSBASE.write_shared(addr.as_u64())
    }
}

const GSBASE: Msr = Msr::new(0xc0000101);
pub struct GS;
impl GS {
    pub fn get() -> VAddr {
        unsafe { VAddr::new_unchecked(GSBASE.read()) }
    }
    pub unsafe fn set(addr: VAddr) {
        GSBASE.write_shared(addr.as_u64())
    }
}

const KERNEL_GSBASE: Msr = Msr::new(0xc0000102);
pub struct KernelGs;
impl KernelGs {
    pub fn get() -> VAddr {
        unsafe { VAddr::new_unchecked(KERNEL_GSBASE.read()) }
    }
    pub unsafe fn set(addr: VAddr) {
        KERNEL_GSBASE.write_shared(addr.as_u64())
    }
}
