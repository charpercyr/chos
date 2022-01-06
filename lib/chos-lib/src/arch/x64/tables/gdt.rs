use core::convert::TryInto;
use core::mem::size_of;

use bit_field::BitField;

use super::{Tss, DescriptorRegister};
use crate::arch::intr::IoPl;
use crate::init::ConstInit;

#[repr(C, align(8))]
#[derive(Debug)]
pub struct Gdt<const N: usize> {
    null: NullDescriptor,
    pub descriptors: [Descriptor; N],
}

impl<const N: usize> Gdt<N> {
    pub const fn new() -> Self {
        Self {
            null: NullDescriptor::new(),
            descriptors: [Descriptor::INIT; N],
        }
    }

    pub unsafe fn load(this: &'static Self) {
        let reg = DescriptorRegister::new(this);
        asm! {
            "ldt ({})",
            in(reg) &reg,
            options(att_syntax, nostack),
        }
    }
}
impl<const N: usize> ConstInit for Gdt<N> {
    const INIT: Self = Self::new();
}

#[derive(Debug)]
pub struct NullDescriptor(u64);

impl NullDescriptor {
    pub const fn new() -> Self {
        Self(0)
    }
}
impl ConstInit for NullDescriptor {
    const INIT: Self = Self::new();
}

#[derive(Debug)]
pub struct Descriptor(u64);

impl Descriptor {
    pub const fn new() -> Self {
        Self(0)
    }

    pub fn set_code64(&mut self, iopl: IoPl) {
        *self = Descriptor::INIT;
        // Accessed
        self.0.set_bit(40, true);
        // Writable
        self.0.set_bit(41, true);
        // Executable
        self.0.set_bit(43, true);
        // User
        self.0.set_bit(44, true);
        // IOPL
        self.0.set_bits(45..47, u8::from(iopl).get_bits(0..2) as u64);
        // Present
        self.0.set_bit(47, true);
        // 64-bit
        self.0.set_bit(53, true);
    }

    pub fn set_data64(&mut self, iopl: IoPl) {
        *self = Descriptor::INIT;
        // Accessed
        self.0.set_bit(40, true);
        // Writable
        self.0.set_bit(41, true);
        // User
        self.0.set_bit(44, true);
        // IOPL
        self.0.set_bits(45..47, u8::from(iopl).get_bits(0..2) as u64);
        // Present
        self.0.set_bit(47, true);
        // 64-bit
        self.0.set_bit(53, true);
    }

    pub fn set_tss(descs: &mut [Self], tss: &'static Tss) {
        assert_eq!(descs.len(), 2);
        let descs: &mut [Self; 2] = unsafe { descs.try_into().unwrap_unchecked() };
        *descs = [Descriptor::INIT; 2];
        
        let tss = tss as *const Tss as u64;
        // Base
        descs[0].0.set_bits(16..40, tss.get_bits(0..24));
        descs[0].0.set_bits(56..64, tss.get_bits(24..32));
        descs[1].0.set_bits(0..32, tss.get_bits(32..64));
        // Limit
        descs[0].0.set_bits(0..16, size_of::<Tss>() as u64 - 1);
        // Type = 9
        descs[0].0.set_bits(40..44, 0b1001);
        // Present
        descs[0].0.set_bit(47, true);
    }
}
impl ConstInit for Descriptor {
    const INIT: Self = Self::new();
}
