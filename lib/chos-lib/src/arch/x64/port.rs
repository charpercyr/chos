use core::arch::asm;
use core::marker::PhantomData;

use crate::access::*;

pub trait PortData: Sized {
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

    pub unsafe fn read(&self) -> T
    where
        A: ReadAccess,
    {
        T::read(self.port)
    }

    pub unsafe fn write(&mut self, value: T)
    where
        A: WriteAccess,
    {
        T::write(self.port, value)
    }
}

pub type PortWriteOnly<T> = Port<T, WriteOnly>;
pub type PortReadOnly<T> = Port<T, ReadOnly>;
