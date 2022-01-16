use core::{arch::asm, mem::size_of};

use crate::arch::mm::VAddr;

#[repr(C, packed)]
pub struct Tss {
    _res0: u32,
    pub rsp: [VAddr; 3],
    _res1: u64,
    pub ist: [VAddr; 7],
    _res2: [u16; 5],
    pub iobp_off: u16,
}

impl Tss {
    pub const fn new() -> Self {
        Self {
            _res0: 0,
            rsp: [VAddr::null(); 3],
            _res1: 0,
            ist: [VAddr::null(); 7],
            _res2: [0; 5],
            iobp_off: size_of::<Self>() as u16,
        }
    }

    pub unsafe fn load(segment: u16) {
        asm! {
            "ltr {:x}",
            in(reg) segment,
            options(nostack),
        }
    }
}
