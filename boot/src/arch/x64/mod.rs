mod asm;
mod intr;
mod mpstart;
mod panic;
mod timer;
mod util;

use core::ffi::c_void;
use core::fmt::{self, Arguments, Write};
use core::ptr::null;
use core::slice;

use chos_lib::arch::acpi::{Rsdt, Xsdp};
use chos_lib::arch::mm::{FrameSize4K, OffsetMapper, VAddr, PAGE_SIZE64};
use chos_lib::arch::qemu::{exit_qemu, QemuStatus};
use chos_lib::arch::serial::Serial;
use chos_lib::fmt::Bytes;
use chos_lib::int::CeilDiv;
use chos_lib::log::{println, set_handler, with_logger, LogHandler, LogLevel, TermColorLogHandler};
use chos_lib::mm::{VFrame, VFrameRange};
use chos_lib::sync::{SpinOnceCell, Spinlock};
use uefi::prelude::*;
use uefi::proto::console::serial::Serial as UefiSerial;
use uefi::proto::media::file::{Directory, File, FileAttribute, FileInfo, FileMode, FileType};
use uefi::proto::media::fs::SimpleFileSystem;
use uefi::table::boot::{AllocateType, MemoryType};
use uefi::table::cfg::ACPI2_GUID;
use uefi::Completion;

use self::mpstart::{is_mp_ready, start_mp};
use self::util::BigBuf;

pub const MEMORY_TYPE_KERNEL: MemoryType = MemoryType::custom(0x8000_0000);
pub const MEMORY_TYPE_MPSTART: MemoryType = MemoryType::custom(0x8000_0001);
pub const MEMORY_TYPE_PAGE_TABLE: MemoryType = MemoryType::custom(0x8000_0002);
pub const MEMORY_TYPE_CMDLINE: MemoryType = MemoryType::custom(0x8000_0003);
pub const MEMORY_TYPE_MODULE: MemoryType = MemoryType::custom(0x8000_0004);

struct UefiSerialWrite<'a> {
    serial: &'a mut UefiSerial<'a>,
}
impl Write for UefiSerialWrite<'_> {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        self.serial
            .write(s.as_bytes())
            .map(Completion::unwrap)
            .map_err(|_| fmt::Error)
    }
}

struct UefiSerialLogger<'a> {
    serial: Spinlock<UefiSerialWrite<'a>>,
}
impl LogHandler for UefiSerialLogger<'_> {
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

static mut BIG_BUF: BigBuf<{ 4 * 4096 }> = BigBuf::new();

struct SerialLog {
    serial: SpinOnceCell<Spinlock<Serial>>,
}
impl LogHandler for SerialLog {
    fn log(&self, fmt: Arguments, _: LogLevel) {
        if let Some(serial) = self.serial.try_get() {
            serial.lock().write_fmt(fmt).unwrap()
        }
    }

    unsafe fn log_unsafe(&self, fmt: Arguments, _: LogLevel) {
        if let Some(serial) = self.serial.try_get() {
            (&mut *serial.get_ptr()).write_fmt(fmt).unwrap()
        }
    }
}

fn read_file_in_memory(
    root: &mut Directory,
    boot_services: &BootServices,
    name: &str,
    mem_ty: MemoryType,
) -> Option<(VFrameRange<FrameSize4K>, u64)> {
    let mut buf: BigBuf<1024> = BigBuf::new();
    let file = root
        .open(name, FileMode::Read, FileAttribute::empty())
        .ok()?
        .unwrap();
    let mut file = if let FileType::Regular(file) = file.into_type().unwrap().unwrap() {
        file
    } else {
        panic!("{} is not a file", name);
    };
    let info = file.get_info::<FileInfo>(buf.inner_mut()).unwrap().unwrap();
    let file_size = info.file_size();
    if file_size == 0 {
        return None;
    }
    let count = file_size.ceil_div(PAGE_SIZE64);
    let base = boot_services
        .allocate_pages(AllocateType::AnyPages, mem_ty, count as usize)
        .unwrap()
        .unwrap();
    let base: VFrame<FrameSize4K> = VFrame::new(VAddr::new(base));
    let range = VFrameRange::new(base, base.add(count));
    let buf =
        unsafe { slice::from_raw_parts_mut(range.start().addr().as_mut_ptr(), file_size as usize) };
    let mut left = buf.len();
    let mut start = 0;
    while left > 0 {
        let read = file.read(&mut buf[start..]).unwrap().unwrap();
        left -= read;
        start += read;
    }
    Some((range, info.file_size()))
}

#[entry]
fn main(image: Handle, mut system_table: SystemTable<Boot>) -> Status {
    let serial = unsafe {
        &mut *system_table
            .boot_services()
            .locate_protocol::<UefiSerial>()?
            .unwrap()
            .get()
    };
    let logger = TermColorLogHandler::new(UefiSerialLogger {
        serial: Spinlock::new(UefiSerialWrite { serial: serial }),
    });
    struct BootData<'a> {
        rsdt: Rsdt<'a>,
    }
    unsafe {
        let BootData { rsdt } = with_logger(&logger, || {
            let rsdt = find_rsdt(&system_table).expect("Should have found RSDT");
            let fs = unsafe {
                &mut *system_table
                    .boot_services()
                    .locate_protocol::<SimpleFileSystem>()
                    .unwrap()
                    .unwrap()
                    .get()
            };
            let mut root = fs.open_volume().unwrap().unwrap();
            let (kernel, kernel_size) = read_file_in_memory(
                &mut root,
                system_table.boot_services(),
                "chos.elf",
                MEMORY_TYPE_KERNEL,
            )
            .expect("There should be a kernel");
            println!(
                "Loaded kernel to {:#012x}-{:#012x} {}",
                kernel.start(),
                kernel.end(),
                Bytes(kernel_size)
            );

            if let Some((cmdline, cmdline_size)) = read_file_in_memory(
                &mut root,
                system_table.boot_services(),
                "cmdline.txt",
                MEMORY_TYPE_CMDLINE,
            ) {
                println!(
                    "Loaded cmdline to {:#012x}-{:#012x} {}",
                    cmdline.start(),
                    cmdline.end(),
                    Bytes(cmdline_size)
                );
            }

            BootData { rsdt }
        });

        let (mut system_table, memory_map) = system_table
            .exit_boot_services(image, BIG_BUF.inner_mut())?
            .expect("Memory map too big");
        let serial = Serial::com1().defaults();
        static SERIAL: TermColorLogHandler<SerialLog> = TermColorLogHandler::new(SerialLog {
            serial: SpinOnceCell::new(),
        });

        SERIAL.inner().serial.get_or_set(Spinlock::new(serial));
        set_handler(&SERIAL);

        let madt = rsdt.madt().expect("No MADT");
        let hpet = rsdt.hpet().expect("No HPET");

        if !is_mp_ready(memory_map, madt) {
            panic!("Cannot start MP");
        }
        intr::initalize(madt);
        timer::initialize(hpet);
        start_mp(madt, |id, user| loop {}, null());
    }
    exit_qemu(QemuStatus::Success);
}
