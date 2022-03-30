mod serial;
mod vga;

use core::fmt::{Arguments, Write};

use chos_lib::log::{LogHandler, TermColorLogHandler};
use chos_lib::sync::spin::lock::Spinlock;

pub trait Output: Write + Send {
    fn init(&mut self);
}

pub static LOCK: Spinlock<()> = Spinlock::new(());
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
    unsafe { chos_lib::log::set_handler(&BOOT_LOG_HANDLER) };
}

fn log(args: Arguments<'_>) {
    let _guard = LOCK.lock();
    if let Some(output) = unsafe { &mut OUTPUT } {
        write!(*output, "{}", args).unwrap();
    }
}

unsafe fn log_unsafe(args: Arguments<'_>) {
    if let Some(output) = &mut OUTPUT {
        write!(*output, "{}", args).unwrap();
    }
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
    }}
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
    }}
}

pub fn hexdump(b: &[u8]) {
    let len = b.len();
    let mut i = 0;
    while i < len {
        let mut j = 0;
        chos_lib::log::print!("[{:016p}]", &b[i]);
        while j < 16 && i < len {
            chos_lib::log::print!(" {:02x}", unsafe { core::ptr::read_volatile(&b[i]) });
            i += 1;
            j += 1;
        }
        chos_lib::log::println!();
    }
}

pub struct BootLogHandler;

impl LogHandler for BootLogHandler {
    fn log(&self, args: Arguments<'_>, _: chos_lib::log::LogLevel) {
        self::log(args)
    }
    unsafe fn log_unsafe(&self, args: Arguments<'_>, _: chos_lib::log::LogLevel) {
        self::log_unsafe(args)
    }
}

pub const BOOT_LOG_HANDLER: TermColorLogHandler<BootLogHandler> =
    TermColorLogHandler::new(BootLogHandler);
