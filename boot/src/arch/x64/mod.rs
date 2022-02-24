mod panic;

use core::ffi::c_void;
use core::fmt::{self, Arguments, Write};
use core::ptr::null_mut;

use chos_lib::arch::acpi::{Rsdt, Xsdp};
use chos_lib::log::{println, with_logger, LogHandler, LogLevel, TermColorLogHandler};
use chos_lib::sync::Spinlock;
use uefi::prelude::*;
use uefi::proto::console::serial::Serial;
use uefi::proto::pi::mp::MpServices;
use uefi::table::cfg::ACPI2_GUID;
use uefi::Completion;

struct SerialWrite<'a> {
    serial: &'a mut Serial<'a>,
}
impl Write for SerialWrite<'_> {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        self.serial
            .write(s.as_bytes())
            .map(Completion::unwrap)
            .map_err(|_| fmt::Error)
    }
}

struct SerialLogger<'a> {
    serial: Spinlock<SerialWrite<'a>>,
}
impl LogHandler for SerialLogger<'_> {
    fn log(&self, args: Arguments, _: LogLevel) {
        let mut serial = self.serial.lock_nodisable();
        serial.write_fmt(args).expect("Should not fail");
    }
    unsafe fn log_unsafe(&self, args: Arguments, _: LogLevel) {
        let serial = &mut *self.serial.get_ptr();
        serial.write_fmt(args).expect("Should not fail");
    }
}

fn find_rsdt(system_table: &SystemTable<Boot>) -> Option<Rsdt<'static>> {
    system_table.config_table().iter().find_map(|cfg| {
        (cfg.guid == ACPI2_GUID).then(|| unsafe { (&*cfg.address.cast::<Xsdp>()).rsdp().rsdt() })
    })
}

extern "efiapi" fn secondary_main(_: *mut c_void) {
    println!("Hello");
    loop {}
}

#[entry]
fn main(_image: Handle, mut system_table: SystemTable<Boot>) -> Status {
    let serial = unsafe {
        &mut *system_table
            .boot_services()
            .locate_protocol::<Serial>()?
            .unwrap()
            .get()
    };
    let logger = TermColorLogHandler::new(SerialLogger {
        serial: Spinlock::new(SerialWrite { serial: serial }),
    });
    unsafe {
        with_logger(&logger, || {
            let rsdt = find_rsdt(&system_table).expect("Should have found RSDT");
            let mp = unsafe {
                &mut *system_table
                    .boot_services()
                    .locate_protocol::<MpServices>()?
                    .unwrap()
                    .get()
            };
            let n_proc = mp.get_number_of_processors()?.unwrap();
            println!("TOTAL: {}\nEN   : {}", n_proc.total, n_proc.enabled);
            // mp.startup_this_ap(1, secondary_main, null_mut(), None).unwrap();
            loop {}
        })
    }
}
