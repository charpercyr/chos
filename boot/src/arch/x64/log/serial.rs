use chos_lib::arch::serial::SerialDyn;

impl super::Output for SerialDyn {
    fn init(&mut self) {
        unsafe { SERIAL.init().expect("Should only be called once") };
    }
}

const COM1_BASE: u16 = 0x3f8;
pub static mut SERIAL: SerialDyn = SerialDyn::new(COM1_BASE);
