use bitflags::bitflags;
use modular_bitfield::specifiers::*;
use modular_bitfield::{bitfield, BitfieldSpecifier};

use crate::access::*;

type Register<T, P> = crate::PaddedVolatile<T, P, 0x10>;
type Reserved = Register<!, NoAccess>;

#[bitfield(bits = 32)]
#[derive(Clone, Copy, Debug)]
pub struct LApicId {
    #[skip]
    __: B24,
    pub lapic_id: u8,
}

#[bitfield(bits = 32)]
#[derive(Clone, Copy, Debug)]
pub struct LApicVersion {
    pub version: u8,
    #[skip]
    __: B8,
    pub max_lvt_entry: u8,
    pub eoi_broadcast_support: bool,
    #[skip]
    __: B7,
}

bitflags! {
    pub struct ErrorStatus: u32 {
        const SEND_CHECKSUM_ERROR = 0b0000_0001;
        const RECV_CHECKSUM_ERROR = 0b0000_0010;
        const SEND_ACCEPT_ERROR = 0b0000_0100;
        const RECV_ACCEPT_ERROR = 0b0000_1000;
        const REDIRECTABLE_IPI = 0b0001_0000;
        const SEND_ILLEGAL_VECTOR = 0b0010_0000;
        const RECV_ILLEGAL_VECTOR = 0b0100_0000;
        const ILLEGAL_REGISTER_ADDRESS = 0b1000_0000;
    }
}

#[derive(BitfieldSpecifier, Clone, Copy, Debug)]
#[bits = 3]
pub enum DeliveryMode {
    Fixed = 0b000,
    Smi = 0b010,
    Nmi = 0b100,
    Init = 0b101,
    ExtInt = 0b111,
}

#[derive(BitfieldSpecifier, Clone, Copy, Debug, PartialEq, Eq)]
#[bits = 1]
pub enum DeliveryStatus {
    Idle = 0,
    Pending = 1,
}

#[derive(BitfieldSpecifier, Clone, Copy, Debug, PartialEq, Eq)]
#[bits = 1]
pub enum InterruptMask {
    Enabled = 0,
    Disabled = 1,
}

#[bitfield(bits = 32)]
#[derive(Clone, Copy, Debug)]
pub struct Interrupt {
    pub vector: u8,
    pub delivery_mode: DeliveryMode,
    #[skip]
    __: B1,
    pub delivery_status: DeliveryStatus,
    #[skip]
    __: B3,
    pub mask: InterruptMask,
    #[skip]
    __: B15,
}

#[derive(BitfieldSpecifier, Clone, Copy, Debug)]
#[bits = 1]
pub enum ApicEnabled {
    Disabled = 0,
    Enabled = 1,
}

#[derive(BitfieldSpecifier, Clone, Copy, Debug)]
#[bits = 1]
pub enum FocusEnabled {
    Disabled = 0,
    Enabled = 1,
}

#[derive(BitfieldSpecifier, Clone, Copy, Debug)]
#[bits = 1]
pub enum EoiBroadcastEnabled {
    Disabled = 0,
    Enabled = 1,
}

#[bitfield(bits = 32)]
#[derive(Clone, Copy, Debug)]
pub struct SpuriousInterrupt {
    pub vector: u8,
    pub enabled: ApicEnabled,
    pub focus: FocusEnabled,
    #[skip]
    __: B2,
    pub eoi_broadcast: EoiBroadcastEnabled,
    #[skip]
    __: B19,
}

#[derive(BitfieldSpecifier, Clone, Copy, Debug)]
#[bits = 2]
pub enum TimerMode {
    OneShot = 0b00,
    Periodic = 0b01,
    TscDeadline = 0b10,
}

#[bitfield(bits = 32)]
#[derive(Clone, Copy, Debug)]
pub struct TimerInterrupt {
    pub vector: u8,
    #[skip]
    __: B4,
    pub delivery_status: DeliveryStatus,
    #[skip]
    __: B3,
    pub mask: InterruptMask,
    pub mode: TimerMode,
    #[skip]
    __: B13,
}

#[bitfield(bits = 32)]
#[derive(Clone, Copy, Debug)]
pub struct LocalInterrupt {
    pub vector: u8,
    pub delivery_mode: DeliveryMode,
    #[skip]
    __: B1,
    pub delivery_status: DeliveryStatus,
    pub pin_polarity: bool,
    pub remote_irr: bool,
    pub trigger_mode: TriggerMode,
    pub mask: InterruptMask,
    #[skip]
    __: B15,
}

#[bitfield(bits = 32)]
#[derive(Clone, Copy, Debug)]
pub struct ErrorInterrupt {
    pub vector: u8,
    #[skip]
    __: B4,
    pub delivery_status: DeliveryStatus,
    #[skip]
    __: B3,
    pub mask: InterruptMask,
    #[skip]
    __: B15,
}

#[derive(BitfieldSpecifier, Clone, Copy, Debug, PartialEq, Eq)]
#[bits = 3]
pub enum CommandDeliveryMode {
    Fixed = 0b000,
    LowestPriority = 0b001,
    Smi = 0b010,
    Nmi = 0b100,
    Init = 0b101,
    StartUp = 0b110,
}

#[derive(BitfieldSpecifier, Clone, Copy, Debug, PartialEq, Eq)]
#[bits = 1]
pub enum DestinationMode {
    Physical = 0,
    Logical = 1,
}

#[derive(BitfieldSpecifier, Clone, Copy, Debug, PartialEq, Eq)]
#[bits = 1]
pub enum Level {
    DeAssert = 0,
    Assert = 1,
}

#[derive(BitfieldSpecifier, Clone, Copy, Debug, PartialEq, Eq)]
#[bits = 1]
pub enum TriggerMode {
    Edge = 0,
    Level = 1,
}

#[derive(BitfieldSpecifier, Clone, Copy, Debug, PartialEq, Eq)]
#[bits = 2]
pub enum DestinationShorthand {
    Destination = 0b00,
    Self_ = 0b01,
    All = 0b10,
    AllButSelf = 0b11,
}

#[bitfield(bits = 32)]
#[derive(Clone, Copy, Debug)]
pub struct CommandRegister {
    pub vector: u8,
    pub delivery_mode: CommandDeliveryMode,
    pub destination_mode: DestinationMode,
    pub delivery_status: DeliveryStatus,
    #[skip]
    __: B1,
    pub level: Level,
    pub trigger_mode: TriggerMode,
    #[skip]
    __: B2,
    pub destination_shorthand: DestinationShorthand,
    #[skip]
    __: B12,
}

#[bitfield(bits = 32)]
#[derive(Clone, Copy, Debug)]
pub struct DestinationRegister {
    #[skip] __: B24,
    pub destination: u8,
}

#[repr(C)]
pub struct CommandRegisters {
    pub cmd: Register<CommandRegister, ReadWrite>,
    pub dst: Register<DestinationRegister, ReadWrite>,
}

#[repr(C)]
pub struct ApicRegisters {
    _res0: [Reserved; 2],
    pub lapic_id: Register<LApicId, ReadWrite>,
    pub lapic_version: Register<LApicVersion, ReadOnly>,
    _res1: [Reserved; 4],
    pub task_priority: Register<u32, ReadWrite>,
    pub arbritation_priority: Register<u32, ReadOnly>,
    pub processor_priority: Register<u32, ReadOnly>,
    pub eoi: Register<u32, WriteOnly>,
    pub remote_read: Register<u32, ReadOnly>,
    pub logical_destination: Register<u32, ReadWrite>,
    pub destination_format: Register<u32, ReadWrite>,
    pub spurious_interrupt_vector: Register<SpuriousInterrupt, ReadWrite>,
    pub in_service: [Register<u32, ReadOnly>; 8],
    pub trigger_mode: [Register<u32, ReadOnly>; 8],
    pub interrupt_request: [Register<u32, ReadOnly>; 8],
    pub error_status: Register<ErrorStatus, ReadOnly>,
    _res2: [Reserved; 6],
    pub lvt_corrected_machine_check_interrupt: Register<Interrupt, ReadWrite>,
    pub interrupt_command: CommandRegisters,
    pub lvt_timer: Register<TimerInterrupt, ReadWrite>,
    pub lvt_thermal_sensor: Register<Interrupt, ReadWrite>,
    pub lvt_performance_monitoring_counters: Register<Interrupt, ReadWrite>,
    pub lvt_lint0: Register<LocalInterrupt, ReadWrite>,
    pub lvt_lint1: Register<LocalInterrupt, ReadWrite>,
    pub lvt_error: Register<ErrorInterrupt, ReadWrite>,
    pub initial_count: Register<u32, ReadWrite>,
    pub current_count: Register<u32, ReadOnly>,
    _res3: [Reserved; 4],
    pub divide_config: Register<u32, ReadWrite>,
    _res4: [Reserved; 1],
}
static_assertions::const_assert_eq!(core::mem::size_of::<ApicRegisters>(), 0x400);
