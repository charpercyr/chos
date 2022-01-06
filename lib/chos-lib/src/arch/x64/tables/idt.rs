use core::arch::asm;
use core::convert::TryInto;
use core::fmt;
use core::intrinsics::transmute;
use core::marker::PhantomData;
use core::mem::size_of;
use core::ops::{Deref, Index, IndexMut};

use bit_field::BitField;
use bitflags::bitflags;

use super::DescriptorRegister;
use crate::arch::intr::IoPl;
use crate::arch::mm::VAddr;
use crate::arch::regs::{Flags, CS};
use crate::Volatile;

#[repr(C)]
#[derive(Debug)]
pub struct InterruptStackFrameValue {
    pub ip: VAddr,
    pub cs: usize,
    pub flags: Flags,
    pub sp: VAddr,
    pub ss: usize,
}

#[repr(C)]
#[derive(Debug)]
pub struct InterruptStackFrame {
    value: InterruptStackFrameValue,
}

impl InterruptStackFrame {
    pub unsafe fn as_mut(&mut self) -> &mut Volatile<InterruptStackFrameValue> {
        transmute(&mut self.value)
    }
}

impl Deref for InterruptStackFrame {
    type Target = InterruptStackFrameValue;
    fn deref(&self) -> &Self::Target {
        &self.value
    }
}

bitflags! {
    #[repr(transparent)]
    pub struct PageFaultError : u64 {
        const PROTECTION_VIOLATION =    1 << 0;
        const CAUSED_BY_WRITE =         1 << 1;
        const USER_MODE =               1 << 2;
        const MALFORMED_TABLE =         1 << 3;
        const INSTRUCTION_FETCH =       1 << 4;
        const PROTECTION_KEY =          1 << 5;
        const SHADOW_STACK =            1 << 6;
        const SGX =                     1 << 7;
        const RMP =                     1 << 8;
    }
}

pub type Handler = extern "x86-interrupt" fn(InterruptStackFrame);
pub type NoReturnHandler = extern "x86-interrupt" fn(InterruptStackFrame) -> !;
pub type HandlerWithError = extern "x86-interrupt" fn(InterruptStackFrame, u64);
pub type NoReturnHandlerWithError = extern "x86-interrupt" fn(InterruptStackFrame, u64) -> !;
pub type PageFaultHandler = extern "x86-interrupt" fn(InterruptStackFrame, PageFaultError);

mod private {
    pub trait Sealed {}
}
pub trait HandlerFn: private::Sealed {
    fn to_u64(self) -> u64;
}
macro_rules! handler_fns {
    ($($h:ty),* $(,)?) => {
        $(
            impl private::Sealed for $h {}
            impl HandlerFn for $h {
                #[inline]
                fn to_u64(self) -> u64 {
                    self as u64
                }
            }
        )*
    };
}
handler_fns!(
    Handler,
    NoReturnHandler,
    HandlerWithError,
    NoReturnHandlerWithError,
    PageFaultHandler,
);

#[repr(C, align(8))]
#[derive(Clone, Debug)]
pub struct Idt {
    /* 00 */ pub divide_error: Entry<Handler>,
    /* 01 */ pub debug: Entry<Handler>,
    /* 02 */ pub non_maskable_interrupt: Entry<Handler>,
    /* 03 */ pub breakpoint: Entry<Handler>,
    /* 04 */ pub overflow: Entry<Handler>,
    /* 05 */ pub bound_range_exceeded: Entry<Handler>,
    /* 06 */ pub invalid_opcode: Entry<Handler>,
    /* 07 */ pub device_not_available: Entry<Handler>,
    /* 08 */ pub double_fault: Entry<NoReturnHandlerWithError>,
    /* 09 */ coprocessor_segment_overrun: Entry<Handler>,
    /* 10 */ pub invalid_tss: Entry<HandlerWithError>,
    /* 11 */ pub segment_not_present: Entry<HandlerWithError>,
    /* 12 */ pub stack_segment_fault: Entry<HandlerWithError>,
    /* 13 */ pub general_protection_fault: Entry<HandlerWithError>,
    /* 14 */ pub page_fault: Entry<PageFaultHandler>,
    res1: Entry<Handler>,
    /* 16 */ pub x87_floating_point: Entry<Handler>,
    /* 17 */ pub alignment_check: Entry<HandlerWithError>,
    /* 18 */ pub machine_check: Entry<NoReturnHandler>,
    /* 19 */ pub simd_floating_point: Entry<Handler>,
    /* 20 */ pub virtualization: Entry<Handler>,
    res2: [Entry<Handler>; 8],
    /* 29 */ pub vmm_communication_exception: Entry<HandlerWithError>,
    /* 30 */ pub security_exception: Entry<HandlerWithError>,
    res3: Entry<Handler>,
    interrupts: [Entry<Handler>; 256 - 32],
}
static_assertions::const_assert_eq!(size_of::<Idt>(), 4096);

impl Idt {
    pub const fn empty() -> Self {
        Self {
            divide_error: Entry::missing(),
            debug: Entry::missing(),
            non_maskable_interrupt: Entry::missing(),
            breakpoint: Entry::missing(),
            overflow: Entry::missing(),
            bound_range_exceeded: Entry::missing(),
            invalid_opcode: Entry::missing(),
            device_not_available: Entry::missing(),
            double_fault: Entry::missing(),
            coprocessor_segment_overrun: Entry::missing(),
            invalid_tss: Entry::missing(),
            segment_not_present: Entry::missing(),
            stack_segment_fault: Entry::missing(),
            general_protection_fault: Entry::missing(),
            page_fault: Entry::missing(),
            res1: Entry::missing(),
            x87_floating_point: Entry::missing(),
            alignment_check: Entry::missing(),
            machine_check: Entry::missing(),
            simd_floating_point: Entry::missing(),
            virtualization: Entry::missing(),
            res2: [Entry::missing(); 8],
            vmm_communication_exception: Entry::missing(),
            security_exception: Entry::missing(),
            res3: Entry::missing(),
            interrupts: [Entry::missing(); 256 - 32],
        }
    }

    pub unsafe fn load(this: &'static Self) {
        let reg = DescriptorRegister::new(this);
        asm! {
            "lidt ({})",
            in(reg) &reg,
            options(att_syntax, nostack),
        }
    }
}

impl Index<usize> for Idt {
    type Output = Entry<Handler>;
    fn index(&self, index: usize) -> &Self::Output {
        match index {
            0..=31 => panic!("Reserved interrupt, use structure member if available"),
            32..=255 => &self.interrupts[index - 32],
            _ => panic!("Out of bounds of Idt: {}", index),
        }
    }
}

impl IndexMut<usize> for Idt {
    fn index_mut(&mut self, index: usize) -> &mut Self::Output {
        match index {
            0..=31 => panic!("Reserved interrupt, use structure member if available"),
            32..=255 => &mut self.interrupts[index - 32],
            _ => panic!("Out of bounds of Idt: {}", index),
        }
    }
}

#[derive(Clone, Copy)]
pub struct EntryOptions(u16);
impl EntryOptions {
    const fn empty() -> Self {
        Self(0b0000_1110_0000_0000)
    }

    pub fn set_present(&mut self, present: bool) -> &mut Self {
        self.0.set_bit(15, present);
        self
    }

    pub fn set_iopl(&mut self, iopl: IoPl) -> &mut Self {
        let iopl: u8 = iopl.into();
        self.0.set_bits(13..15, iopl.get_bits(0..2) as u16);
        self
    }

    pub fn enable_interrupts(&mut self, enabled: bool) -> &mut Self {
        self.0.set_bit(8, enabled);
        self
    }

    pub fn set_stack_index(&mut self, idx: Option<u16>) -> &mut Self {
        self.0.set_bits(0..3, idx.map(|idx| idx + 1).unwrap_or(0));
        self
    }
}

impl fmt::Debug for EntryOptions {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let iopl: IoPl = (self.0.get_bits(13..15) as u8).try_into().unwrap();
        let stack_index = match self.0.get_bits(0..3) {
            0 => None,
            idx => Some(idx - 1),
        };
        f.debug_struct("EntryOptions")
            .field("present", &self.0.get_bit(15))
            .field("iopl", &iopl)
            .field("interrupts_enabled", &self.0.get_bit(8))
            .field("stack_index", &stack_index)
            .finish()
    }
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct Entry<H> {
    pointer_low: u16,
    gdt_selector: u16,
    pub options: EntryOptions,
    pointer_mid: u16,
    pointer_hig: u32,
    _res: u32,
    handler: PhantomData<H>,
}
static_assertions::const_assert_eq!(size_of::<Entry<Handler>>(), 16);

impl<H> Entry<H> {
    pub const fn missing() -> Self {
        Self {
            pointer_low: 0,
            gdt_selector: 0,
            options: EntryOptions::empty(),
            pointer_mid: 0,
            pointer_hig: 0,
            _res: 0,
            handler: PhantomData,
        }
    }

    pub fn set_handler(&mut self, h: H) -> &mut EntryOptions
    where
        H: HandlerFn,
    {
        let h = h.to_u64();
        self.pointer_low = h.get_bits(0..16) as u16;
        self.pointer_mid = h.get_bits(16..32) as u16;
        self.pointer_hig = h.get_bits(32..64) as u32;

        let cs = CS::get();
        self.gdt_selector = cs;

        self.options.set_present(true)
    }
}

impl<H> fmt::Debug for Entry<H> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut pointer = 0u64;
        pointer.set_bits(0..16, self.pointer_low as u64);
        pointer.set_bits(16..32, self.pointer_mid as u64);
        pointer.set_bits(32..64, self.pointer_hig as u64);
        f.debug_struct("Entry")
            .field("pointer", &pointer)
            .field("gdt_selector", &self.gdt_selector)
            .field("options", &self.options)
            .finish()
    }
}