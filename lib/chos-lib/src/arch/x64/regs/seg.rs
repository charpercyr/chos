use core::arch::asm;

use crate::arch::mm::VAddr;
use crate::arch::msr::Msr;

#[derive(Debug)]
pub struct CS;

impl CS {
    pub fn read() -> u16 {
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
    pub fn read() -> VAddr {
        unsafe { VAddr::new_unchecked(FSBASE.read_raw()) }
    }
    pub unsafe fn write(addr: VAddr) {
        FSBASE.write_raw_shared(addr.as_u64())
    }
}

const GSBASE: Msr = Msr::new(0xc0000101);
pub struct GS;
impl GS {
    pub fn read() -> VAddr {
        unsafe { VAddr::new_unchecked(GSBASE.read_raw()) }
    }
    pub unsafe fn write(addr: VAddr) {
        GSBASE.write_raw_shared(addr.as_u64())
    }
}

const KERNEL_GSBASE: Msr = Msr::new(0xc0000102);
pub struct KernelGs;
impl KernelGs {
    pub fn read() -> VAddr {
        unsafe { VAddr::new_unchecked(KERNEL_GSBASE.read_raw()) }
    }
    pub unsafe fn write(addr: VAddr) {
        KERNEL_GSBASE.write_raw_shared(addr.as_u64())
    }
}
