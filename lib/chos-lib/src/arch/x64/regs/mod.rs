mod cr;
mod flags;
mod seg;

use core::arch::asm;

pub use cr::*;
pub use flags::*;
pub use seg::*;

use super::mm::VAddr;

pub struct Rsp;

impl Rsp {
    #[inline(always)]
    pub fn read_raw() -> u64 {
        let rsp;
        unsafe {
            asm! {
                "mov %rsp, {rsp}",
                rsp = lateout(reg) rsp,
                options(att_syntax, nomem, nostack),
            }
        }
        rsp
    }

    #[inline(always)]
    pub fn read() -> VAddr {
        unsafe { VAddr::new_unchecked(Self::read_raw()) }
    }
}

#[repr(C)]
#[derive(Debug, Clone)]
pub struct IntrRegs {
    pub error: u64,
    pub rip: VAddr,
    pub cs: u64,
    pub rflags: Flags,
    pub rsp: VAddr,
    pub ss: u64,
}

#[repr(C)]
#[derive(Debug, Clone)]
pub struct ScratchRegs {
    pub rax: u64,
    pub r11: u64,
    pub r10: u64,
    pub r9: u64,
    pub r8: u64,
    pub rcx: u64,
    pub rdx: u64,
    pub rsi: u64,
    pub rdi: u64,
    pub intr: IntrRegs,
}

#[repr(C)]
#[derive(Debug, Clone)]
pub struct AllRegs {
    pub r15: u64,
    pub r14: u64,
    pub r13: u64,
    pub r12: u64,
    pub rbp: u64,
    pub rbx: u64,
    pub unsaved: ScratchRegs,
}