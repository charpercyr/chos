use core::{fmt, hint};

use x86_64::instructions::port::Port;

pub struct Serial {
    ports: [Port<u8>; 6],
}

impl Serial {
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
        }
    }

    unsafe fn init(&mut self) {
        self.ports[1].write(0x00);
        self.ports[3].write(0x80);
        self.ports[0].write(0x03);
        self.ports[1].write(0x00);
        self.ports[3].write(0x03);
        self.ports[2].write(0xc7);
        self.ports[4].write(0x0b);
    }

    fn put(&mut self, c: u8) {
        unsafe {
            while self.ports[5].read() & 0x20 == 0 {
                hint::spin_loop();
            }
            self.ports[0].write(c);
        }
    }
}

impl fmt::Write for Serial {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        for &b in s.as_bytes() {
            self.put(b);
        }
        Ok(())
    }
}

impl super::Output for Serial {
    fn init(&mut self) {
        unsafe { self.init() };
    }
}

const COM1_BASE: u16 = 0x3f8;
pub static mut SERIAL: Serial = Serial::new(COM1_BASE);
