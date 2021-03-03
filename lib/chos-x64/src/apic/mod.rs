
use x86_64::VirtAddr;
use x86_64::registers::model_specific::Msr;

const MSR_LOCAL_APIC_BASE: u32 = 0x1b;

mod reg;
use reg::*;

pub mod timer;

pub struct Apic {
    registers: *mut ApicRegisters,
}

impl Apic {
    pub unsafe fn from_msr() -> Self {
        Self::from_msr_offset(0)
    }

    pub unsafe fn from_msr_offset(offset: usize) -> Self {
        let base = Msr::new(MSR_LOCAL_APIC_BASE);
        let base = base.read() as usize;
        Self::with_address(base + offset)
    }

    pub fn with_address(addr: usize) -> Self {
        Self {
            registers: addr as _,
        }
    }

    pub unsafe fn initialize(&mut self, spurious: u8) {
        let regs = self.registers_mut();
        let value = 0x100 | (spurious as u32);

        regs.lvt_corrected_machine_check_interrupt.disable();
        regs.lvt_error.disable();
        regs.lvt_lint0.disable();
        regs.lvt_lint1.disable();
        regs.lvt_performance_monitoring_counters.disable();
        regs.lvt_thermal_sensor.disable();
        regs.lvt_timer.disable();

        regs.spurious_interrupt_vector.write(value);
    }

    pub fn base(&self) -> VirtAddr {
        unsafe { VirtAddr::new_unsafe(self.registers as u64) }
    }

    pub unsafe fn id(&self) -> u32 {
        self.registers().lapic_id.read()  
    }

    pub unsafe fn version(&self) -> u32 {
        (*self.registers).lapic_version.read()
    }
    
    pub unsafe fn timer(&mut self) -> timer::Timer<'_> {
        timer::Timer::new(self.registers_mut())
    }

    unsafe fn registers(&self) -> &ApicRegisters {
        &*self.registers
    }

    unsafe fn registers_mut(&mut self) -> &mut ApicRegisters {
        &mut *self.registers
    }
}