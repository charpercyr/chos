use core::arch::asm;
use core::convert::TryFrom;

use modular_bitfield::BitfieldSpecifier;

use super::regs::Flags;

pub struct IntrStatus(Flags);

pub fn disable_interrups() {
    unsafe {
        asm! {
            "cli",
            options(nomem, nostack),
        };
    }
}

pub fn enable_interrupts() {
    unsafe {
        asm! {
            "sti",
            options(nomem, nostack),
        }
    }
}

pub fn disable_interrups_save() -> IntrStatus {
    let flags = IntrStatus(Flags::get());
    disable_interrups();
    flags
}

pub fn restore_interrupts(status: IntrStatus) {
    if status.0.intr_enable() {
        enable_interrupts();
    }
}

pub fn without_interrupts<R, F: FnOnce() -> R>(f: F) -> R {
    let flags = disable_interrups_save();
    let res = f();
    restore_interrupts(flags);
    res
}

pub fn breakpoint() {
    unsafe {
        asm! { "int3" }
    }
}

pub macro int($n:expr) {
    unsafe {
        core::arch::asm!(
            "int {}",
            const $n,
        );
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, BitfieldSpecifier)]
#[bits = 2]
pub enum IoPl {
    Ring0 = 0,
    Ring1 = 1,
    Ring2 = 2,
    Ring3 = 3,
}
impl IoPl {
    pub const KERNEL: Self = Self::Ring0;
    pub const USER: Self = Self::Ring3;
}
impl From<IoPl> for u8 {
    fn from(iopl: IoPl) -> Self {
        use IoPl::*;
        match iopl {
            Ring0 => 0,
            Ring1 => 1,
            Ring2 => 2,
            Ring3 => 3,
        }
    }
}
#[derive(Debug, Clone, Copy)]
pub struct InvalidIoPl;
impl TryFrom<u8> for IoPl {
    type Error = InvalidIoPl;
    fn try_from(value: u8) -> Result<Self, Self::Error> {
        match value {
            0 => Ok(Self::Ring0),
            1 => Ok(Self::Ring1),
            2 => Ok(Self::Ring2),
            3 => Ok(Self::Ring3),
            _ => Err(InvalidIoPl),
        }
    }
}
