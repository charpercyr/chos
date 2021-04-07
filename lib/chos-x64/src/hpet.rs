use core::fmt;
use core::marker::PhantomData;
use core::mem::MaybeUninit;

use chos_lib::{bitfield::*, PaddedVolatile, ReadOnly, ReadWrite, Volatile};

bitfield! {
    #[derive(Copy, Clone)]
    struct GeneralCapabilities(u64) {
        [imp Debug]
        counter_clk_period: 63, 32 -> u32;
        vendor_id: 32, 16 -> u16;
        leg_rt_cap: 15;
        count_size_cap: 13;
        num_tim_cap: 12, 8 -> u8;
        rev_id: 7, 0 -> u8;
    }

    #[derive(Copy, Clone)]
    struct Configuration(u64) {
        [imp Debug]
        leg_rt, set_leg_rt: 1;
        enable, set_enable: 0;
    }

    #[derive(Copy, Clone)]
    struct TimerConfiguration(u64) {
        [imp Debug]
        int_route_cap: 63, 32 -> u32;
        fsb_int_del_cap: 15;
        fsb_en_cnf, set_fsb_en_cnf: 14;
        int_route_cnf, set_int_route_cnf: 13, 9 -> u8;
        mode_32_cnf, set_mode_32_cnf: 8;
        val_set_cnf, set_val_set_cnf: 6;
        size_cap: 5;
        per_int_cap: 4;
        type_cnf, set_type_cnf: 3;
        int_enb_cnf, set_int_enb_cnf: 2;
        int_type_cnf, set_int_type_cnf: 1;
    }
}

#[repr(C, packed)]
struct Registers {
    capabilities: PaddedVolatile<GeneralCapabilities, ReadOnly, 0x10>,
    configuration: PaddedVolatile<Configuration, ReadWrite, 0x10>,
    interrupt_status: PaddedVolatile<u32, ReadWrite, 0x10>,
    _res: MaybeUninit<[u8; 0xc0]>,
    main_counter_value: PaddedVolatile<u64, ReadWrite, 0x10>,
}

#[allow(safe_packed_borrows)]
#[repr(C, packed)]
#[derive(Debug)]
struct TimerRegisters {
    configuration: Volatile<TimerConfiguration, ReadWrite>,
    comparator: Volatile<u64, ReadWrite>,
    interrupt_route: Volatile<u64, ReadWrite>,
}

impl fmt::Debug for Registers {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Registers")
            .field("capabilities", unsafe { &self.capabilities })
            .field("configuration", unsafe { &self.configuration })
            .field("interrupt_status", unsafe { &self.interrupt_status })
            .field("counter", unsafe { &self.main_counter_value })
            .finish()
    }
}

#[derive(Debug)]
pub struct HPET {
    registers: &'static mut Registers,
}

impl HPET {
    pub unsafe fn with_address(address: usize) -> Self {
        Self {
            registers: &mut *(address as *mut Registers),
        }
    }

    pub fn period(&self) -> u32 {
        unsafe { self.registers.capabilities.read().counter_clk_period() as u32 }
    }

    pub fn vendor_id(&self) -> u16 {
        unsafe { self.registers.capabilities.read().vendor_id() as u16 }
    }

    pub fn timer_count(&self) -> u8 {
        unsafe { self.registers.capabilities.read().num_tim_cap() as u8 + 1 }
    }

    pub fn enabled(&self) -> bool {
        unsafe { self.registers.configuration.read().enable() }
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
        unsafe { self.registers.main_counter_value.read() }
    }

    pub unsafe fn set_count(&mut self, count: u64) {
        self.registers.main_counter_value.write(count)
    }

    pub fn get_timer(&mut self, n: u8) -> Timer<'_> {
        let n = n as usize;
        let offset = 0x100 + 0x20 * n;
        let base: *mut u8 = (self.registers as *mut Registers).cast();
        unsafe {
            let ptr: *mut TimerRegisters = base.add(offset).cast();
            Timer {
                registers: &mut *ptr,
                hpet: PhantomData,
            }
        }
    }
}

pub struct Timer<'a> {
    registers: &'a mut TimerRegisters,
    hpet: PhantomData<&'a mut HPET>,
}

impl Timer<'_> {
    pub fn int_route(&self) -> u8 {
        unsafe { self.registers.configuration.read().int_route_cnf() }
    }

    pub unsafe fn set_int_route(&mut self, route: u8) {
        self.registers.configuration.update(|config| {
            config.set_int_route_cnf(route);
        })
    }

    pub unsafe fn set_comparator(&mut self, value: u64) {
        self.registers.comparator.write(value)
    }

    pub unsafe fn set_enabled(&mut self, enabled: bool) {
        self.registers.configuration.update(|config| {
            config.set_int_enb_cnf(enabled);
        });
    }

    pub unsafe fn enable(&mut self) {
        self.set_enabled(true);
    }

    pub unsafe  fn disable(&mut self) {
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
