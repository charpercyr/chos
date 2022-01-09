use core::fmt;
use core::hint::spin_loop;
use core::marker::PhantomData;
use core::mem::replace;

use super::port::Port;

pub struct Uninit;
pub struct Init;

pub struct Serial<S> {
    ports: [Port<u8>; 6],
    state: PhantomData<S>,
}

impl Serial<Uninit> {
    pub const fn new(base: u16) -> Self {
        Self {
            ports: [
                Port::new(base),
                Port::new(base + 1),
                Port::new(base + 2),
                Port::new(base + 3),
                Port::new(base + 4),
                Port::new(base + 5),
            ],
            state: PhantomData,
        }
    }

    pub unsafe fn init(mut self) -> Serial<Init> {
        self.ports[1].write(0x00);
        self.ports[3].write(0x80);
        self.ports[0].write(0x03);
        self.ports[1].write(0x00);
        self.ports[3].write(0x03);
        self.ports[2].write(0xc7);
        self.ports[4].write(0x0b);
        Serial {
            ports: self.ports,
            state: PhantomData,
        }
    }
}

impl Serial<Init> {
    pub fn put(&mut self, c: u8) {
        unsafe {
            while self.ports[5].read() & 0x20 == 0 {
                spin_loop();
            }
            self.ports[0].write(c);
        }
    }
}

impl fmt::Write for Serial<Init> {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        for &b in s.as_bytes() {
            self.put(b);
        }
        Ok(())
    }
}

enum SerialState {
    Uninit(Serial<Uninit>),
    Tmp,
    Init(Serial<Init>),
}

pub struct SerialDyn {
    state: SerialState,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct AlreadyInitializedError;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct NotInitializedError;

impl SerialDyn {
    pub const fn new(port: u16) -> Self {
        Self {
            state: SerialState::Uninit(Serial::new(port)),
        }
    }

    pub unsafe fn init(&mut self) -> Result<(), AlreadyInitializedError> {
        match replace(&mut self.state, SerialState::Tmp) {
            SerialState::Uninit(serial) => {
                self.state = SerialState::Init(serial.init());
                Ok(())
            },
            SerialState::Init(serial) => {
                self.state = SerialState::Init(serial);
                Err(AlreadyInitializedError)
            },
            SerialState::Tmp => unreachable!(),
        }
    }

    pub fn put(&mut self, c: u8) -> Result<(), NotInitializedError> {
        if let SerialState::Init(serial) = &mut self.state {
            serial.put(c);
            Ok(())
        } else {
            Err(NotInitializedError)
        }
    }

    pub fn init_mut(&mut self) -> Result<&mut Serial<Init>, NotInitializedError> {
        if let SerialState::Init(serial) = &mut self.state {
            Ok(serial)
        } else {
            Err(NotInitializedError)
        }
    }
}

impl fmt::Write for SerialDyn {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        let inner = self.init_mut().map_err(|_| fmt::Error)?;
        for c in s.bytes() {
            inner.put(c);
        }
        Ok(())
    }
}
