mod command;
mod interrupt;
mod reg;

pub use command::*;
pub use interrupt::*;
pub use reg::*;

use crate::cpumask::Cpumask;
use crate::Volatile;
use crate::mm::VAddr;

pub struct Apic<'a> {
    regs: &'a mut reg::ApicRegisters,
}

impl Apic<'_> {
    pub unsafe fn new(addr: VAddr) -> Self {
        Self {
            regs: addr.as_mut(),
        }
    }

    pub fn address(&self) -> VAddr {
        VAddr::from(&*self.regs)
    }

    pub fn id(&self) -> u8 {
        self.regs.lapic_id.read().lapic_id()
    }

    pub fn lint0_mut(&mut self) -> &mut Volatile<LocalInterrupt> {
        self.regs.lvt_lint0.as_volatile_mut()
    }

    pub fn lint1_mut(&mut self) -> &mut Volatile<LocalInterrupt> {
        self.regs.lvt_lint1.as_volatile_mut()
    }

    pub unsafe fn initialize(&mut self) {
        self.initialize_with_spurious_vector(0xff);
    }

    pub unsafe fn initialize_with_spurious_vector(&mut self, vector: u8) {
        self.regs
            .lvt_corrected_machine_check_interrupt
            .update(|lvt| lvt.set_mask(InterruptMask::Disabled));
        self.regs
            .lvt_error
            .update(|lvt| lvt.set_mask(InterruptMask::Disabled));
        self.regs
            .lvt_lint0
            .update(|lvt| lvt.set_mask(InterruptMask::Disabled));
        self.regs
            .lvt_lint1
            .update(|lvt| lvt.set_mask(InterruptMask::Disabled));
        self.regs
            .lvt_performance_monitoring_counters
            .update(|lvt| lvt.set_mask(InterruptMask::Disabled));
        self.regs
            .lvt_thermal_sensor
            .update(|lvt| lvt.set_mask(InterruptMask::Disabled));
        self.regs
            .lvt_timer
            .update(|lvt| lvt.set_mask(InterruptMask::Disabled));

        self.regs.spurious_interrupt_vector.write(
            SpuriousInterrupt::new()
                .with_enabled(ApicEnabled::Enabled)
                .with_vector(vector),
        );
        let mask = Cpumask::cpu(self.id());
        self.regs
            .logical_destination
            .write(DestinationRegister::new().with_destination(mask.raw() as u8));
    }

    pub unsafe fn eoi(&mut self) {
        self.regs.eoi.write(0);
    }

    pub fn commands(&mut self) -> InterruptCommand<'_> {
        InterruptCommand {
            regs: &mut self.regs.interrupt_command,
        }
    }
}
