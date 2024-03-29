use core::fmt;
use core::marker::PhantomData;
use core::mem::MaybeUninit;

use modular_bitfield::specifiers::*;
use modular_bitfield::{bitfield, BitfieldSpecifier};

use crate::mm::VAddr;
use crate::{PaddedVolatile, ReadOnly, ReadWrite, Volatile};

#[bitfield(bits = 64)]
#[derive(Copy, Clone, Debug)]
struct GeneralCapabilities {
    rev_id: u8,
    num_tim_cap: B5,
    count_size_cap: bool,
    #[skip]
    __: B1,
    leg_rt_cap: bool,
    vendor_id: u16,
    counter_clk_period: u32,
}

#[bitfield(bits = 64)]
#[derive(Copy, Clone, Debug)]
struct Configuration {
    enable: bool,
    leg_rt: bool,
    #[skip]
    __: B62,
}

#[derive(BitfieldSpecifier, Copy, Clone, Debug, PartialEq, Eq)]
#[bits = 1]
pub enum TimerType {
    OneShot = 0,
    Periodic = 1,
}

#[bitfield(bits = 64)]
#[derive(Copy, Clone, Debug)]
struct TimerConfiguration {
    #[skip]
    __: B1,
    int_typ: bool,
    int_enb_cnf: bool,
    type_cnf: TimerType,
    per_int_cap: bool,
    size_cap: bool,
    val_set_cnf: bool,
    #[skip]
    __: B1,
    mode_32_cnf: bool,
    int_route_cnf: B5,
    fst_en_cnf: bool,
    fsb_int_del_cap: bool,
    #[skip]
    __: B16,
    int_route_cap: u32,
}

#[repr(C)]
struct Registers {
    capabilities: PaddedVolatile<GeneralCapabilities, ReadOnly, 0x10>,
    configuration: PaddedVolatile<Configuration, ReadWrite, 0x10>,
    interrupt_status: PaddedVolatile<u32, ReadWrite, 0x10>,
    _res: MaybeUninit<[u8; 0xc0]>,
    main_counter_value: PaddedVolatile<u64, ReadWrite, 0x10>,
}

#[repr(C)]
#[derive(Debug)]
struct TimerRegisters {
    configuration: Volatile<TimerConfiguration, ReadWrite>,
    comparator: Volatile<u64, ReadWrite>,
    interrupt_route: Volatile<u64, ReadWrite>,
}

impl fmt::Debug for Registers {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Registers")
            .field("capabilities", &self.capabilities)
            .field("configuration", &self.configuration)
            .field("interrupt_status", &self.interrupt_status)
            .field("counter", &self.main_counter_value)
            .finish()
    }
}

pub struct Hpet {
    registers: &'static mut Registers,
}

impl Hpet {
    pub unsafe fn new(address: VAddr) -> Self {
        Self {
            registers: address.as_mut(),
        }
    }

    pub fn period(&self) -> u32 {
        self.registers.capabilities.read().counter_clk_period()
    }

    pub fn vendor_id(&self) -> u16 {
        self.registers.capabilities.read().vendor_id()
    }

    pub fn timer_count(&self) -> u8 {
        self.registers.capabilities.read().num_tim_cap() + 1
    }

    pub fn enabled(&self) -> bool {
        self.registers.configuration.read().enable()
    }

    pub unsafe fn enable(&mut self) {
        self.set_enabled(true)
    }

    pub unsafe fn disable(&mut self) {
        self.set_enabled(false)
    }

    pub unsafe fn set_enabled(&mut self, e: bool) {
        self.registers.configuration.update(|config| {
            config.set_enable(e);
        })
    }

    pub fn count(&self) -> u64 {
        self.registers.main_counter_value.read()
    }

    pub unsafe fn set_count(&mut self, count: u64) {
        self.registers.main_counter_value.write(count)
    }

    pub fn get_timer(&self, n: u8) -> Timer<'_> {
        let n = n as usize;
        let offset = 0x100 + 0x20 * n;
        let base: *const u8 = (self.registers as *const Registers).cast();
        unsafe {
            let ptr: *const TimerRegisters = base.add(offset).cast();
            Timer {
                registers: &*ptr,
                hpet: PhantomData,
            }
        }
    }

    pub fn get_timer_mut(&mut self, n: u8) -> TimerMut<'_> {
        let n = n as usize;
        let offset = 0x100 + 0x20 * n;
        let base: *mut u8 = (self.registers as *mut Registers).cast();
        unsafe {
            let ptr: *mut TimerRegisters = base.add(offset).cast();
            TimerMut {
                registers: &mut *ptr,
                hpet: PhantomData,
            }
        }
    }
}

impl fmt::Debug for Hpet {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        struct TimersDebug<'a>(&'a Hpet);
        impl fmt::Debug for TimersDebug<'_> {
            fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
                let mut dbg = f.debug_list();
                for i in 0..self.0.timer_count() {
                    dbg.entry(&self.0.get_timer(i));
                }
                dbg.finish()
            }
        }
        let tim_dbg = TimersDebug(self);
        f.debug_struct("Hpet")
            .field("registers", self.registers)
            .field("timers", &tim_dbg)
            .finish()
    }
}

pub struct Timer<'a> {
    registers: &'a TimerRegisters,
    hpet: PhantomData<&'a Hpet>,
}

pub struct TimerMut<'a> {
    registers: &'a mut TimerRegisters,
    hpet: PhantomData<&'a mut Hpet>,
}

impl TimerMut<'_> {
    pub fn int_route(&self) -> u8 {
        self.registers.configuration.read().int_route_cnf()
    }

    pub unsafe fn set_int_route(&mut self, route: u8) {
        self.registers.configuration.update(|config| {
            config.set_int_route_cnf(route);
        })
    }

    pub fn int_route_mask(&self) -> u32 {
        self.registers.configuration.read().int_route_cap()
    }

    pub unsafe fn set_comparator(&mut self, value: u64) {
        self.registers.comparator.write(value)
    }

    pub unsafe fn set_type(&mut self, typ: TimerType) {
        self.registers
            .configuration
            .update(|config| config.set_type_cnf(typ))
    }

    pub unsafe fn set_enabled(&mut self, enabled: bool) {
        self.registers.configuration.update(|config| {
            config.set_int_enb_cnf(enabled);
        });
    }

    pub unsafe fn enable(&mut self) {
        self.set_enabled(true);
    }

    pub unsafe fn disable(&mut self) {
        self.set_enabled(false);
    }
}

impl fmt::Debug for Timer<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Timer")
            .field("registers", &self.registers)
            .finish()
    }
}

impl fmt::Debug for TimerMut<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("TimerMut")
            .field("registers", &self.registers)
            .finish()
    }
}
