use crate::int::IntSplit;

pub struct Msr(u32);

impl Msr {
    pub const fn new(reg: u32) -> Self {
        Self(reg)
    }

    pub unsafe fn read(&self) -> u64 {
        let vh: u32;
        let vl: u32;
        asm! {
            "rdmsr",
            in("ecx") self.0,
            lateout("edx") vh,
            lateout("eax") vl,
            options(nostack, nomem),
        }
        u64::join(vh, vl)
    }

    pub unsafe fn write(&mut self,v : u64) {
        self.write_shared(v)
    }

    pub unsafe fn write_shared(&self, v: u64) {
        let (vh, vl) = v.split();
        asm! {
            "wrmsr",
            in("ecx") self.0,
            in("edx") vh,
            in("eax") vl,
            options(nostack, nomem),
        }
    }
}
