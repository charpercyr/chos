use core::arch::asm;

use bitflags::bitflags;

use crate::arch::mm::{FrameSize4K, PAddr, VAddr, PAGE_MASK};
use crate::mm::PFrame;

macro cr_read($reg:expr) {{
    let cr: u64;
    asm! {
        concat!("mov %", $reg, ", {cr}"),
        cr = lateout(reg) cr,
        options(att_syntax, nomem, nostack),
    }
    cr
}}

macro cr_write($reg:expr, $cr:expr) {{
    let cr: u64 = $cr;
    asm! {
        concat!("mov {cr}, %", $reg),
        cr = in(reg) cr,
        options(att_syntax, nomem, nostack),
    }
}}

bitflags! {
    pub struct Cr0Flags: u64 {
        const PROTECTED_MODE_ENABLE = 1 << 0;
        const MONITOR_CO_PROCESSOR = 1 << 1;
        const EMULATION = 1 << 2;
        const TASK_SWITCHED = 1 << 3;
        const EXTENSION_TYPE = 1 << 4;
        const NUMERIC_ERROR = 1 << 5;
        const WRITE_PROTECT = 1 << 16;
        const ALIGNMENT_MASK = 1 << 18;
        const NOT_WRITE_THROUGH = 1 << 29;
        const CACHE_DISABLE = 1 << 30;
        const PAGING = 1 << 31;
    }
}

pub struct Cr0;
impl Cr0 {
    pub fn read_raw() -> u64 {
        unsafe { cr_read!("cr0") }
    }

    pub fn read() -> Cr0Flags {
        Cr0Flags::from_bits_truncate(Self::read_raw())
    }

    pub unsafe fn write_raw(cr0: u64) {
        cr_write!("cr0", cr0)
    }

    pub unsafe fn write(cr0: Cr0Flags) {
        Self::write_raw(cr0.bits())
    }
}

pub struct Cr2;
impl Cr2 {
    pub fn read_raw() -> u64 {
        unsafe { cr_read!("cr2") }
    }

    pub fn read() -> VAddr {
        unsafe { VAddr::new_unchecked(Self::read_raw()) }
    }
}

bitflags! {
    pub struct Cr3Flags: u64 {
        const PAGE_LEVEL_WRITETHROUGH = 1 << 3;
        const PAGE_LEVEL_CACHE_DISABLE = 1 << 4;
    }
}

pub struct Cr3;
impl Cr3 {
    pub fn read_raw() -> u64 {
        unsafe { cr_read!("cr3") }
    }

    pub fn read() -> (PFrame<FrameSize4K>, Cr3Flags) {
        let cr3 = Self::read_raw();
        let addr = unsafe { PFrame::new_unchecked(PAddr::new(cr3 & !PAGE_MASK)) };
        let flags = Cr3Flags::from_bits_truncate(cr3);
        (addr, flags)
    }

    pub unsafe fn write_raw(cr3: u64) {
        cr_write!("cr3", cr3)
    }

    pub unsafe fn write(addr: PFrame<FrameSize4K>, flags: Cr3Flags) {
        let cr3 = addr.addr().as_u64() | flags.bits();
        Self::write_raw(cr3);
    }
}

bitflags! {
    pub struct Cr4Flags: u64 {
        const VIRTUAL_8086_MODE = 1 << 0;
        const PROTECTED_MODE_VIRTUAL_INTERRUPTS = 1 << 1;
        const TSC_RING0 = 1 << 2;
        const DEBUGGING = 1 << 3;
        const PAGE_SIZE_EXTENSION = 1 << 4;
        const PHYSICAL_ADDRESS_EXTENSION = 1 << 5;
        const MACHINE_CHECK_EXCEPTION = 1 << 6;
        const PAGE_GLOBAL_ENABLE = 1 << 7;
        const PERFORMANCE_MONITORING_COUNTER_ENABLE = 1 << 8;
        const FXSAVE_FXRSTOR = 1 << 9;
        const SIMD_EXCEPTIONS = 1 << 10;
        const USER_MODE_INSTRUCTION_PREVENTION = 1 << 11;
        const VIRTUAL_MACHINE_EXTENTIONS = 1 << 13;
        const SAFER_MODE_EXTENSIONS = 1 << 14;
        const FS_GS_BASE = 1 << 16;
        const PCIDE = 1 << 17;
        const OSXSAVE = 1 << 18;
        const SUPERVISOR_MODE_EXECUTIONS_PROTECTION = 1 << 20;
        const SUPERVISOR_MODE_ACCESS_PROTECTION = 1 << 21;
        const USER_PROTECTION_KEYS = 1 << 22;
        const CONTROL_FLOW_ENFORCEMENT = 1 << 23;
        const SUPERVISOR_PROTECTION_KEYS = 1 << 24;
    }
}

pub struct Cr4;
impl Cr4 {
    pub fn read_raw() -> u64 {
        unsafe { cr_read!("cr4") }
    }

    pub fn read() -> Cr4Flags {
        Cr4Flags::from_bits_truncate(Self::read_raw())
    }

    pub unsafe fn write_raw(cr4: u64) {
        cr_write!("cr4", cr4)
    }

    pub unsafe fn write(cr4: Cr0Flags) {
        Self::write_raw(cr4.bits())
    }
}