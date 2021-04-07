
use core::fmt;

use chos_lib::bitfield::*;

bitfield! {
    #[derive(Copy, Clone)]
    #[repr(transparent)]
    struct RedirectionEntryInner(u64) {
        [imp Debug]
        [imp Eq]
        destination, set_destination: 63, 56 -> u8;
        mask, set_mask: 16;
        trigger_mode, set_trigger_mode: 15 -> TriggerMode;
        remote_irr: 14;
        pin_polarity, set_pin_polarity: 13 -> PinPolarity;
        delivery_status: 12 -> DeliveryStatus;
        destination_mode, set_destination_mode: 11;
        delivery_mode, set_delivery_mode: 10, 8 -> DeliveryMode;
        vector, set_vector: 7, 0 -> u8;
    }
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
            inner: RedirectionEntryInner::new(value),
        }
    }

    pub fn bits(&self) -> u64 {
        self.inner.bits
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
        self.inner.set_vector(v)
    }

    pub fn delivery_mode(&self) -> DeliveryMode {
        self.inner.delivery_mode()
    }

    pub fn set_delivery_mode(&mut self, d: DeliveryMode) {
        self.inner.set_delivery_mode(d)
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
            },
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

chos_lib::field_enum! {
    #[derive(Copy, Clone, Debug, PartialEq, Eq)]
    pub enum DeliveryMode (u64) {
        Fixed = 0,
        LowPriority = 1,
        SMI = 2,
        NMI = 4,
        Init = 5,
        ExInt = 6,
    }

    #[derive(Copy, Clone, Debug, PartialEq, Eq)]
    pub enum DeliveryStatus (u64) {
        Idle = 0,
        Pending = 1,
    }

    #[derive(Copy, Clone, Debug, PartialEq, Eq)]
    pub enum PinPolarity (u64) {
        HighActive = 0,
        LowActive = 1,
    }

    #[derive(Copy, Clone, Debug, PartialEq, Eq)]
    pub enum TriggerMode (u64) {
        Edge = 0,
        Level = 1,
    }
}