
use core::mem::transmute;
use core::ptr::addr_of;

use chos_lib::{ReadWrite, WriteOnly, NoAccess};

use static_assertions as sa;

mod redirection;
pub use redirection::*;

type Register<P> = chos_lib::PaddedVolatile<u32, P, 0x10>;
sa::const_assert_eq!(core::mem::size_of::<Register<NoAccess>>(), 0x10);

#[repr(C, packed)]
struct Registers {
    select: Register<WriteOnly>,
    register: Register<ReadWrite>,
}

pub struct IOApic {
    registers: &'static mut Registers,
}

impl IOApic {
    const ADDR_ID: u32 = 0x0;
    const ADDR_VER: u32 = 0x1;
    const ADDR_RED_BASE: u32 = 0x10;

    pub unsafe fn with_address(addr: usize) -> Self {
        Self {
            registers: &mut *(addr as *mut Registers),
        }
    }

    pub fn apic_id(&mut self) -> u8 {
        ((self.read(Self::ADDR_ID) & 0x0f00_0000) >> 24) as u8
    }

    pub fn version(&mut self) -> u8 {
        (self.read(Self::ADDR_VER) & 0xff) as u8
    }

    pub fn max_red_entries(&mut self) -> u8 {
        ((self.read(Self::ADDR_VER) & 0x00ff_0000) >> 16) as u8 + 1
    }

    pub fn read_redirection(&mut self, n: u8) -> RedirectionEntry {
        let n = n as u32;
        let low = self.read(Self::ADDR_RED_BASE + 2 * n) as u64;
        let high = self.read(Self::ADDR_RED_BASE + 2 * n + 1) as u64;
        let value = (high << 32) | low;
        RedirectionEntry::new(value)
    }

    pub unsafe fn write_redirection(&mut self, n: u8, red: RedirectionEntry) {
        let n = n as u32;
        let value: u64 = transmute(red);
        let high = (value >> 32) as u32;
        let low = (value & 0xffff_ffff) as u32;
        self.write(Self::ADDR_RED_BASE + 2 * n + 1, high);
        self.write(Self::ADDR_RED_BASE + 2 * n, low);
    }

    pub unsafe fn update_redirection<R>(&mut self, n: u8, f: impl FnOnce(&mut RedirectionEntry) -> R) -> R {
        let mut entry = self.read_redirection(n);
        let res = f(&mut entry);
        self.write_redirection(n, entry);
        res
    }

    fn read(&mut self, addr: u32) -> u32 {
        self.registers.select.write(addr);
        self.registers.register.read()
    }

    unsafe fn write(&mut self, addr: u32, value: u32) {
        self.registers.select.write(addr);
        self.registers.register.write(value)
    }
}
