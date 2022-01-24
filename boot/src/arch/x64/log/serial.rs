use chos_lib::arch::serial::SerialDyn;

impl super::Output for SerialDyn {
    fn init(&mut self) {
        unsafe { SERIAL.defaults().expect("Should only be called once") };
    }
}

pub static mut SERIAL: SerialDyn = SerialDyn::com1();
