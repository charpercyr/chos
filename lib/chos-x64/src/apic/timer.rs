
use super::{ApicRegisters};

pub struct Timer<'a> {
    registers: &'a mut ApicRegisters,
}

pub enum TimerMode {
    Periodic,
    OneShot,
}

pub struct TimerConfig {
    pub interrupt: u8,
    pub divisor: u32,
    pub count: u32,
    pub mode: TimerMode,
}

impl<'a> Timer<'a> {
    pub fn new(registers: &'a mut ApicRegisters) -> Self {
        Self { registers }
    }

    pub unsafe fn configure(&mut self, config: &TimerConfig) {
        self.registers.lvt_timer.set_vector_number(config.interrupt);
        self.registers.lvt_timer.enable();
        self.registers.divide_config.write(config.divisor);
        self.registers.initial_count.write(config.count);
    }

    pub fn current_count(&self) -> u32 {
        self.registers.current_count.read()
    }
}
