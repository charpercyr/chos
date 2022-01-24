use core::arch::asm;
use core::marker::PhantomData;

use crate::access::*;

pub trait PortData: Copy {
    unsafe fn read(port: u16) -> Self;
    unsafe fn write(port: u16, value: Self);
}

impl PortData for u8 {
    unsafe fn read(port: u16) -> Self {
        let value;
        asm! {
            "inb %dx, %al",
            in("dx") port,
            lateout("al") value,
            options(att_syntax, nomem, nostack),
        }
        value
    }
    unsafe fn write(port: u16, value: Self) {
        asm! {
            "outb %al, %dx",
            in("dx") port,
            in("al") value,
            options(att_syntax, nomem, nostack),
        }
    }
}

impl PortData for [u8; 1] {
    unsafe fn read(port: u16) -> Self {
        u8::read(port).to_ne_bytes()
    }
    unsafe fn write(port: u16, value: Self) {
        u8::write(port, u8::from_ne_bytes(value))
    }
}

impl PortData for [u8; 2] {
    unsafe fn read(port: u16) -> Self {
        u16::read(port).to_ne_bytes()
    }
    unsafe fn write(port: u16, value: Self) {
        u16::write(port, u16::from_ne_bytes(value))
    }
}

impl PortData for [u8; 4] {
    unsafe fn read(port: u16) -> Self {
        u32::read(port).to_ne_bytes()
    }
    unsafe fn write(port: u16, value: Self) {
        u32::write(port, u32::from_ne_bytes(value))
    }
}

impl PortData for u16 {
    unsafe fn read(port: u16) -> Self {
        let value;
        asm! {
            "inw %dx, %ax",
            in("dx") port,
            lateout("ax") value,
            options(att_syntax, nomem, nostack),
        }
        value
    }
    unsafe fn write(port: u16, value: Self) {
        asm! {
            "outw %ax, %dx",
            in("dx") port,
            in("ax") value,
            options(att_syntax, nomem, nostack),
        }
    }
}

impl PortData for u32 {
    unsafe fn read(port: u16) -> Self {
        let value;
        asm! {
            "inl %dx, %eax",
            in("dx") port,
            lateout("eax") value,
            options(att_syntax, nomem, nostack),
        }
        value
    }
    unsafe fn write(port: u16, value: Self) {
        asm! {
            "outl %eax, %dx",
            in("dx") port,
            in("eax") value,
            options(att_syntax, nomem, nostack),
        }
    }
}

#[repr(transparent)]
pub struct Port<T: PortData, A = ReadWrite> {
    port: u16,
    value: PhantomData<T>,
    access: PhantomData<A>,
}

impl<T: PortData, A> Port<T, A> {
    pub const fn new(port: u16) -> Self {
        Self {
            port,
            value: PhantomData,
            access: PhantomData,
        }
    }

    pub unsafe fn read(&mut self) -> T
    where
        A: ReadAccess,
    {
        self.read_shared()
    }

    pub unsafe fn read_shared(&self) -> T
    where
        A: ReadAccess,
    {
        T::read(self.port)
    }

    pub unsafe fn write(&mut self, value: T)
    where
        A: WriteAccess,
    {
        self.write_shared(value)
    }

    pub unsafe fn write_shared(&self, value: T)
    where
        A: WriteAccess,
    {
        T::write(self.port, value)
    }

    pub unsafe fn update<R>(&mut self, f: impl FnOnce(&mut T) -> R) -> R
    where
        A: WriteAccess + ReadAccess,
    {
        self.update_shared(f)
    }

    pub unsafe fn update_shared<R>(&self, f: impl FnOnce(&mut T) -> R) -> R
    where
        A: WriteAccess + ReadAccess,
    {
        let mut v = self.read_shared();
        let ret = f(&mut v);
        self.write_shared(v);
        ret
    }
}

pub type PortWriteOnly<T> = Port<T, WriteOnly>;
pub type PortReadOnly<T> = Port<T, ReadOnly>;

pub struct PortRange<T: PortData, const N: u16> {
    base: u16,
    value: PhantomData<T>,
}
impl<T: PortData, const N: u16> PortRange<T, N> {
    pub const fn new(base: u16) -> Self {
        Self {
            base,
            value: PhantomData,
        }
    }

    pub unsafe fn read(&self, i: u16) -> T {
        assert!(i < N);
        T::read(self.base + i)
    }

    pub unsafe fn write(&mut self, i: u16, value: T) {
        assert!(i < N);
        T::write(self.base + i, value)
    }

    pub unsafe fn update<R>(&mut self, i: u16, f: impl FnOnce(&mut T) -> R) -> R {
        assert!(i < N);
        let mut v = self.read(i);
        let ret = f(&mut v);
        self.write(i, v);
        ret
    }
}
