use core::hint::spin_loop;
use core::time::Duration;

use super::reg::{
    CommandDeliveryMode, CommandRegister, CommandRegisters, DeliveryStatus, DestinationMode,
    DestinationRegister, DestinationShorthand, Level, TriggerMode,
};
use crate::arch::mm::FrameSize4K;
use crate::cpumask::Cpumask;
use crate::mm::VFrame;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Destination {
    Logical { cpus: Cpumask },
    Physical { id: u8 },
    Self_,
    All,
    AllButSelf,
}

impl Destination {
    pub const fn logical(cpus: Cpumask) -> Self {
        Self::Logical { cpus }
    }
    pub const fn physical(id: u8) -> Self {
        Self::Physical { id }
    }
    pub const fn self_() -> Self {
        Self::Self_
    }
    pub const fn all() -> Self {
        Self::All
    }
    pub const fn all_but_self() -> Self {
        Self::AllButSelf
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Command {
    Fixed { vector: u8 },
    LowestPriority { vector: u8 },
    Smi,
    Nmi,
    Init,
    InitDeassert,
    Sipi { frame: VFrame<FrameSize4K> },
}

impl Command {
    pub const fn fixed(vector: u8) -> Self {
        Self::Fixed { vector }
    }
    pub const fn lowest_priority(vector: u8) -> Self {
        Self::LowestPriority { vector }
    }
    pub const fn smi() -> Self {
        Self::Smi
    }
    pub const fn nmi() -> Self {
        Self::Nmi
    }
    pub const fn init() -> Self {
        Self::Init
    }
    pub const fn init_deassert() -> Self {
        Self::InitDeassert
    }
    pub const fn sipi(frame: VFrame<FrameSize4K>) -> Self {
        Self::Sipi { frame }
    }
}

pub struct InterruptCommand<'a> {
    pub(super) regs: &'a mut CommandRegisters,
}

impl InterruptCommand<'_> {
    pub unsafe fn send(&mut self, dest: Destination, cmd: Command) {
        self.send_nowait(dest, cmd);
        self.wait_for_delivery();
    }

    pub unsafe fn send_nowait(&mut self, dst: Destination, cmd: Command) {
        let mut dst_reg = DestinationRegister::new();
        let mut cmd_reg = CommandRegister::new()
            .with_level(Level::Assert)
            .with_trigger_mode(TriggerMode::Edge);
        match dst {
            Destination::Physical { id } => {
                cmd_reg.set_destination_shorthand(DestinationShorthand::Destination);
                dst_reg.set_destination(id);
                cmd_reg.set_destination_mode(DestinationMode::Physical);
            }
            Destination::Logical { cpus } => {
                cmd_reg.set_destination_shorthand(DestinationShorthand::Destination);
                dst_reg.set_destination(cpus.raw() as u8);
                cmd_reg.set_destination_mode(DestinationMode::Logical);
            }
            Destination::Self_ => cmd_reg.set_destination_shorthand(DestinationShorthand::Self_),
            Destination::All => cmd_reg.set_destination_shorthand(DestinationShorthand::All),
            Destination::AllButSelf => {
                cmd_reg.set_destination_shorthand(DestinationShorthand::AllButSelf)
            }
        }
        match cmd {
            Command::Fixed { vector } => {
                cmd_reg.set_delivery_mode(CommandDeliveryMode::Fixed);
                cmd_reg.set_vector(vector);
            }
            Command::LowestPriority { vector } => {
                cmd_reg.set_delivery_mode(CommandDeliveryMode::LowestPriority);
                cmd_reg.set_vector(vector);
            }
            Command::Smi => cmd_reg.set_delivery_mode(CommandDeliveryMode::Smi),
            Command::Nmi => cmd_reg.set_delivery_mode(CommandDeliveryMode::Nmi),
            Command::Init => cmd_reg.set_delivery_mode(CommandDeliveryMode::Init),
            Command::InitDeassert => {
                cmd_reg.set_delivery_mode(CommandDeliveryMode::Init);
                cmd_reg.set_trigger_mode(TriggerMode::Level);
                cmd_reg.set_level(Level::DeAssert);
            }
            Command::Sipi { frame } => {
                let page = frame.page();
                assert!(page <= 255);
                cmd_reg.set_delivery_mode(CommandDeliveryMode::StartUp);
                cmd_reg.set_vector(page as u8);
            }
        };
        self.regs.dst.write(dst_reg);
        self.regs.cmd.write(cmd_reg);
    }

    pub unsafe fn wait_for_delivery(&mut self) {
        while self.regs.cmd.read().delivery_status() == DeliveryStatus::Pending {
            spin_loop();
        }
    }

    pub unsafe fn start_ap(
        &mut self,
        lapic_id: u8,
        code: VFrame<FrameSize4K>,
        delay: impl Fn(Duration),
    ) {
        let dst = Destination::physical(lapic_id);
        self.send(dst, Command::init());
        self.send(dst, Command::init_deassert());

        delay(Duration::from_millis(10));

        for _ in 0..2 {
            self.send_nowait(dst, Command::sipi(code));
        }
        delay(Duration::from_micros(200));
        self.wait_for_delivery();
    }
}
