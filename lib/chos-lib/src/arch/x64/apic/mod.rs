mod reg;
use core::time::Duration;

use reg::*;

use super::mm::VAddr;

pub struct Apic {
    registers: &'static mut ApicRegisters,
}

impl Apic {
    pub unsafe fn with_address(addr: usize) -> Self {
        Self {
            registers: &mut *(addr as *mut ApicRegisters),
        }
    }

    pub unsafe fn initialize(&mut self, spurious: u8) {
        let value = spurious as u32;

        self.registers
            .lvt_corrected_machine_check_interrupt
            .disable();
        self.registers.lvt_error.disable();
        self.registers.lvt_lint0.disable();
        self.registers.lvt_lint1.disable();
        self.registers.lvt_performance_monitoring_counters.disable();
        self.registers.lvt_thermal_sensor.disable();
        self.registers.lvt_timer.disable();

        self.registers
            .spurious_interrupt_vector
            .write(value | (1 << 8));
    }

    pub fn base(&self) -> VAddr {
        unsafe { VAddr::new_unchecked(self.registers as *const _ as u64) }
    }

    pub unsafe fn id(&self) -> u8 {
        (self.registers.lapic_id.read() >> 24) as u8
    }

    pub unsafe fn version(&self) -> u32 {
        (*self.registers).lapic_version.read()
    }
    pub unsafe fn start_ap(&mut self, lapic_id: u8, code_page: usize, delay_us: fn(Duration)) {
        start_ap(
            &mut self.registers.interrupt_command,
            lapic_id,
            code_page,
            delay_us,
        );
    }

    pub unsafe fn eoi(&mut self) {
        self.registers.eoi.write(0)
    }
}

unsafe fn wait_for_delivery(cmd: &mut Register<crate::ReadWrite>) {
    while cmd.read() & (1 << 12) != 0 {
        core::hint::spin_loop();
    }
}

unsafe fn start_ap(
    cmd: &mut CommandRegisters,
    lapic_id: u8,
    code_page: usize,
    delay: fn(Duration),
) {
    cmd.send_command(lapic_id, Command::Init);
    cmd.send_command(lapic_id, Command::InitDeassert);

    delay(Duration::from_millis(10));

    for _ in 0..2 {
        cmd.send_command_nowait(
            lapic_id,
            Command::Sipi {
                page_number: code_page as u8,
            },
        );
        delay(Duration::from_micros(200));
        cmd.wait_for_delivery();
    }
}
