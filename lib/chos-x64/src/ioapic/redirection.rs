use core::fmt;

use modular_bitfield::specifiers::*;
use modular_bitfield::{bitfield, BitfieldSpecifier};

#[derive(BitfieldSpecifier, Copy, Clone, Debug, PartialEq, Eq)]
#[bits = 1]
pub enum TriggerMode {
    Edge = 0,
    Level = 1,
}

#[derive(BitfieldSpecifier, Copy, Clone, Debug, PartialEq, Eq)]
#[bits = 1]
pub enum DeliveryStatus {
    Idle = 0,
    Pending = 1,
}

#[derive(BitfieldSpecifier, Copy, Clone, Debug, PartialEq, Eq)]
#[bits = 1]
pub enum PinPolarity {
    HighActive = 0,
    LowActive = 1,
}

#[derive(BitfieldSpecifier, Copy, Clone, Debug, PartialEq, Eq)]
#[bits = 3]
pub enum DeliveryMode {
    Fixed = 0,
    LowPriority = 1,
    SMI = 2,
    NMI = 4,
    Init = 5,
    ExInt = 6,
}

#[bitfield(bits = 64)]
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
struct RedirectionEntryInner {
    vector: u8,
    delivery_mode: DeliveryMode,
    destination_mode: bool,
    delivery_status: DeliveryStatus,
    pin_polarity: PinPolarity,
    remote_irr: bool,
    trigger_mode: TriggerMode,
    mask: bool,
    #[skip]
    __: B39,
    destination: u8,
}

#[derive(Copy, Clone, PartialEq, Eq)]
#[repr(transparent)]
pub struct RedirectionEntry {
    inner: RedirectionEntryInner,
}

impl fmt::Debug for RedirectionEntry {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Debug::fmt(&self.inner, f)
    }
}

impl RedirectionEntry {
    pub const fn new(value: u64) -> Self {
        Self {
            inner: RedirectionEntryInner::from_bytes(value.to_ne_bytes()),
        }
    }

    pub fn set_defaults(&mut self) {
        self.set_vector(0);
        self.set_delivery_mode(DeliveryMode::Fixed);
        self.set_destination(Destination::Physical(0));
        self.set_pin_polarity(PinPolarity::HighActive);
        self.set_trigger_mode(TriggerMode::Edge);
        self.disable();
    }

    pub fn vector(&self) -> u8 {
        self.inner.vector()
    }

    pub fn set_vector(&mut self, v: u8) {
        self.inner.set_vector(v);
    }

    pub fn delivery_mode(&self) -> DeliveryMode {
        self.inner.delivery_mode()
    }

    pub fn set_delivery_mode(&mut self, d: DeliveryMode) {
        self.inner.set_delivery_mode(d);
    }

    pub fn destination(&self) -> Destination {
        let v = self.inner.destination();
        match self.inner.destination_mode() {
            false => Destination::Physical(v),
            true => Destination::Logical(v),
        }
    }

    pub fn set_destination(&mut self, d: Destination) {
        let mode: bool;
        let dest: u8;
        match d {
            Destination::Physical(v) => {
                mode = false;
                dest = v;
            }
            Destination::Logical(v) => {
                mode = true;
                dest = v;
            }
        }
        self.inner.set_destination_mode(mode);
        self.inner.set_destination(dest);
    }

    pub fn delivery_status(&self) -> DeliveryStatus {
        self.inner.delivery_status()
    }

    pub fn pin_polarity(&self) -> PinPolarity {
        self.inner.pin_polarity()
    }

    pub fn set_pin_polarity(&mut self, p: PinPolarity) {
        self.inner.set_pin_polarity(p);
    }

    pub fn trigger_mode(&self) -> TriggerMode {
        self.inner.trigger_mode()
    }

    pub fn set_trigger_mode(&mut self, m: TriggerMode) {
        self.inner.set_trigger_mode(m);
    }

    pub fn enabled(&self) -> bool {
        !self.inner.mask()
    }

    pub fn enable(&mut self) {
        self.set_enabled(true);
    }

    pub fn disable(&mut self) {
        self.set_enabled(false);
    }

    pub fn set_enabled(&mut self, e: bool) {
        self.inner.set_mask(!e);
    }
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum Destination {
    Physical(u8),
    Logical(u8),
}
