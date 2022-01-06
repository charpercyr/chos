use core::arch::asm;

#[repr(C, packed)]
pub struct Tss {
    _res0: u32,
    pub rsp: [u64; 3],
    _res1: u64,
    pub ist: [u64; 7],
    _res2: [u16; 5],
    pub iobp_off: u16,
}

pub struct TssArgs {
    pub rsp: [u64; 3],
    pub ist: [u64; 7],
    pub iobp_off: u16,
}

impl Tss {
    pub const fn new(args: TssArgs) -> Self {
        Self {
            _res0: 0,
            rsp: args.rsp,
            _res1: 0,
            ist: args.ist,
            _res2: [0; 5],
            iobp_off: args.iobp_off,
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
