


use chos_lib::{ReadOnly, WriteOnly, ReadWrite, NoAccess, Volatile, WriteAccess, ReadAccess};

use static_assertions as sa;

#[repr(C)]
pub struct Register<P> {
    value: Volatile<u32, P>,
    _res: [u32; 3],
}
sa::const_assert_eq!(core::mem::size_of::<Register<NoAccess>>(), 0x10);

impl<P> Register<P> {
    pub fn read(&self) -> u32 where P: ReadAccess {
        self.value.read()
    }

    pub fn write(&mut self, v: u32) where P: WriteAccess {
        self.value.write(v)
    }
}

#[repr(transparent)]
pub struct InterruptRegister(Register<ReadWrite>);

impl InterruptRegister {
    pub unsafe fn set_vector_number(&mut self, n: u8) {
        let mut value = self.0.read();
        value &= !0xff;
        value |= n as u32;
        self.0.write(value);
    }

    pub unsafe fn disable(&mut self) {
        let mut value = self.0.read();
        value |= 1 << 16;
        self.0.write(value);
    }

    pub unsafe fn enable(&mut self) {
        let mut value = self.0.read();
        value &= !(1 << 16);
        self.0.write(value);
    }
}

#[repr(C)]
pub struct ApicRegisters {
    _res0: [Register<NoAccess>; 2],
    pub lapic_id: Register<ReadWrite>,
    pub lapic_version: Register<ReadOnly>,
    _res1: [Register<NoAccess>; 4],
    pub task_priority: Register<ReadWrite>,
    pub arbritation_priority: Register<ReadOnly>,
    pub processor_priority: Register<ReadOnly>,
    pub eoi: Register<WriteOnly>,
    pub remote_read: Register<ReadOnly>,
    pub logical_destination: Register<ReadWrite>,
    pub destination_format: Register<ReadWrite>,
    pub spurious_interrupt_vector: Register<ReadWrite>,
    pub in_service: [Register<ReadOnly>; 8],
    pub trigger_mode: [Register<ReadOnly>; 8],
    pub interrupt_request: [Register<ReadOnly>; 8],
    pub error_status: Register<ReadOnly>,
    _res2: [Register<NoAccess>; 6],
    pub lvt_corrected_machine_check_interrupt: InterruptRegister,
    pub interrupt_command: [Register<ReadWrite>; 2],
    pub lvt_timer: InterruptRegister,
    pub lvt_thermal_sensor: InterruptRegister,
    pub lvt_performance_monitoring_counters: InterruptRegister,
    pub lvt_lint0: InterruptRegister,
    pub lvt_lint1: InterruptRegister,
    pub lvt_error: InterruptRegister,
    pub initial_count: Register<ReadWrite>,
    pub current_count: Register<ReadOnly>,
    _res3: [Register<NoAccess>; 4],
    pub divide_config: Register<ReadWrite>,
    _res4: [Register<NoAccess>; 1],
}
sa::const_assert_eq!(core::mem::size_of::<ApicRegisters>(), 0x400);