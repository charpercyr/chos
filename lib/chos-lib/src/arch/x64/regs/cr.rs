use core::arch::asm;

use bitflags::bitflags;

use crate::arch::mm::{FrameSize4K, PAddr, VAddr, PAGE_MASK};
use crate::mm::PFrame;

pub struct Cr2;

impl Cr2 {
    pub fn read() -> VAddr {
        let addr: u64;
        unsafe {
            asm! {
                "mov %cr2, {}",
                lateout(reg) addr,
                options(att_syntax, nomem, nostack),
            }
            VAddr::new_unchecked(addr)
        }
    }
}

pub struct Cr3;

bitflags! {
    pub struct Cr3Flags: u64 {
        const PAGE_LEVEL_WRITETHROUGH = 1 << 3;
        const PAGE_LEVEL_CACHE_DISABLE = 1 << 4;
    }
}

impl Cr3 {
    pub fn read() -> (PFrame<FrameSize4K>, Cr3Flags) {
        unsafe {
            let cr3;
            asm! {
                "mov %cr3, {}",
                lateout(reg) cr3,
                options(att_syntax, nomem, nostack),
            }
            let addr = PFrame::new_unchecked(PAddr::new(cr3 & !PAGE_MASK));
            let flags = Cr3Flags::from_bits_truncate(cr3);
            (addr, flags)
        }
    }

    pub unsafe fn write(addr: PFrame<FrameSize4K>, flags: Cr3Flags) {
        let cr3 = addr.addr().as_u64() | flags.bits();
        asm! {
            "mov {}, %cr3",
            in(reg) cr3,
            options(att_syntax, nomem, nostack),
        }
    }
}
