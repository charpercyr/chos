use core::marker::PhantomData;

pub use crate::access::*;
use crate::int::IntSplit;

#[repr(transparent)]
pub struct Msr<P = ReadWrite>(u32, PhantomData<P>);

impl<P> Msr<P> {
    pub const fn new(reg: u32) -> Self {
        Self(reg, PhantomData)
    }

    pub unsafe fn read(&self) -> u64
    where
        P: ReadAccess,
    {
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

    pub unsafe fn write(&mut self, v: u64)
    where
        P: WriteAccess,
    {
        self.write_shared(v)
    }

    pub unsafe fn write_shared(&self, v: u64)
    where
        P: WriteAccess,
    {
        let (vh, vl) = v.split();
        asm! {
            "wrmsr",
            in("ecx") self.0,
            in("edx") vh,
            in("eax") vl,
            options(nostack, nomem),
        }
    }

    pub unsafe fn update(&mut self, f: impl FnOnce(&mut u64))
    where
        P: ReadAccess + WriteAccess,
    {
        self.update_shared(f)
    }

    pub unsafe fn update_shared(&self, f: impl FnOnce(&mut u64))
    where
        P: ReadAccess + WriteAccess,
    {
        let mut v = self.read();
        f(&mut v);
        self.write_shared(v);
    }
}
