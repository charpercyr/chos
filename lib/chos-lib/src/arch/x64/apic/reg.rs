use core::hint::spin_loop;

use crate::{NoAccess, ReadOnly, ReadWrite, WriteOnly};
use static_assertions as sa;

pub type Register<P> = crate::PaddedVolatile<u32, P, 0x10>;
sa::const_assert_eq!(core::mem::size_of::<Register<NoAccess>>(), 0x10);

#[repr(transparent)]
#[derive(Debug)]
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
pub struct CommandRegisters {
    command: Register<ReadWrite>,
    destination: Register<ReadWrite>,
}

pub enum Command {
    Normal { vector: u8 },
    LowPriority { vector: u8 },
    SMI { vector: u8 },
    NMI { vector: u8 },
    Init,
    InitDeassert,
    Sipi { page_number: u8 },
}

impl CommandRegisters {
    pub unsafe fn send_command(&mut self, destination: u8, command: Command) {
        self.send_command_nowait(destination, command);
        self.wait_for_delivery();
    }

    pub unsafe fn send_command_nowait(&mut self, destination: u8, command: Command) {
        use Command::*;
        let mut vector: u32 = 0;
        let cmd: u32;
        let mut init: u32 = 0b01;
        match command {
            Normal { vector: v } => {
                vector = v as _;
                cmd = 0;
            }
            LowPriority { vector: v } => {
                vector = v as _;
                cmd = 1;
            }
            SMI { vector: v } => {
                vector = v as _;
                cmd = 2;
            }
            NMI { vector: v } => {
                vector = v as _;
                cmd = 4;
            }
            Init => {
                cmd = 5;
                init = 0b01;
            }
            InitDeassert => {
                cmd = 5;
                init = 0b10;
            }
            Sipi { page_number } => {
                vector = page_number as _;
                cmd = 6;
            }
        };

        let destination = ((destination as u32) & 0xf) << 24;
        let cmd = vector | (cmd << 8) | (init << 14);

        self.destination.write(destination);
        self.command.write(cmd);
    }

    pub unsafe fn wait_for_delivery(&mut self) {
        while (self.command.read() & (1 << 12)) != 0 {
            spin_loop();
        }
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
    pub interrupt_command: CommandRegisters,
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
