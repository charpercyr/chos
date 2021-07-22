
use modular_bitfield::{bitfield, specifiers::*};

#[bitfield(bits = 64)]
#[derive(Clone, Copy, Debug)]
pub struct Flags {
    pub carry: bool,
    #[skip] __: B1,
    pub parity: bool,
    #[skip] __: B1,
    pub ajust: bool,
    #[skip] __: B1,
    pub zero: bool,
    pub sign: bool,
    pub trap: bool,
    pub intr_enable: bool,
    pub direction: bool,
    pub overflow: bool,
    pub iopl: B2,
    pub nt: bool,
    #[skip] __: B1,
    pub resume: bool,
    pub virtual_8086: bool,
    pub alignment_check: bool,
    pub virtual_intr_flag: bool,
    pub virtual_intr_pending: bool,
    pub id: bool,
    #[skip] __: B42,
}
impl Flags {
    pub fn get() -> Self {
        let flags: u64;
        unsafe {
            asm! {
                "pushf",
                "pop {flags}",
                flags = out(reg) flags,
                options(att_syntax, nomem),
            }
        }
        Self::from_bytes(flags.to_ne_bytes())
    }

    pub unsafe fn set(flags: Self) {
        asm! {
            "push {flags}",
            "popf",
            flags = in(reg) u64::from_ne_bytes(flags.into_bytes()),
            options(att_syntax, nomem),
        }
    }
}
