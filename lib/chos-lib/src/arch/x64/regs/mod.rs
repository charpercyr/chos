
mod cr;
mod flags;
mod seg;

pub use cr::*;
pub use flags::*;
pub use seg::*;

#[repr(C)]
pub struct IntrRegs {
    pub user: u64,
    pub rip: u64,
    pub cs: u64,
    pub flags: Flags,
    pub rsp: u64,
    pub ss: u64,
}

#[repr(C)]
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
pub struct AllRegs {
    pub r15: u64,
    pub r14: u64,
    pub r13: u64,
    pub r12: u64,
    pub rbp: u64,
    pub rbx: u64,
    pub unsaved: ScratchRegs,
}
