mod serial;
mod vga;

use core::fmt::Write;

use spin::Mutex;

pub trait Output: Write + Send {
    fn init(&mut self);
}

pub static LOCK: Mutex<()> = Mutex::new(());
pub static mut OUTPUT: Option<&'static mut dyn Output> = None;

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
#[allow(dead_code)]
pub enum Device {
    Vga,
    Serial,
}

pub fn initialize(dev: Device) {
    let _guard = LOCK.lock();
    let output = unsafe { &mut OUTPUT };
    if output.is_some() {
        panic!("Output already initialized");
    }
    let dev: &mut dyn Output = match dev {
        Device::Vga => unsafe { &mut vga::VGA },
        Device::Serial => unsafe { &mut serial::SERIAL },
    };
    dev.init();
    *output = Some(dev);
}

#[macro_export]
macro_rules! print {
    ($($args:tt)*) => {{
        let _guard = $crate::arch::x64::log::LOCK.lock();
        #[allow(unused_unsafe)]
        if let Some(out) = unsafe { $crate::arch::x64::log::OUTPUT.as_mut() } {
            #[allow(unused_imports)]
            use core::fmt::Write;
            write!(*out, $($args)*).unwrap();
        }
    }};
}

#[macro_export]
macro_rules! println {
    ($($args:tt)*) => {{
        let _guard = $crate::arch::x64::log::LOCK.lock();
        #[allow(unused_unsafe)]
        if let Some(out) = unsafe { $crate::arch::x64::log::OUTPUT.as_mut() } {
            #[allow(unused_imports)]
            use core::fmt::Write;
            writeln!(*out, $($args)*).unwrap();
        }
    }};
}

#[macro_export]
macro_rules! hexdump {
    ($v:expr) => {{
        #[allow(unused_unsafe)]
        unsafe {
            use core::mem::{size_of_val, transmute};
            let len = size_of_val(&$v);
            let ptr: *const u8 = transmute(&$v);
            let mut i = 0;
            while i < len {
                print!("[{:016p}]", ptr);
                let mut j = 0;
                while i < len && j < 16 {
                    print!(" {:02x}", *ptr.offset(i as isize));
                    i += 1;
                    j += 1;
                }
                println!();
            }
        }
    }};
}

#[macro_export]
macro_rules! unsafe_print {
    ($($args:tt)*) => {{
        let out = &mut $crate::arch::x64::log::OUTPUT;
        if let Some(out) = out {
            #[allow(unused_imports)]
            use core::fmt::Write;
            write!(*out, $($args)*).unwrap();
        }
    }};
}

#[macro_export]
macro_rules! unsafe_println {
    ($($args:tt)*) => {{
        let out = &mut $crate::arch::x64::log::OUTPUT;
        if let Some(out) = out {
            #[allow(unused_imports)]
            use core::fmt::Write;
            writeln!(*out, $($args)*).unwrap();
        }
    }};
}
