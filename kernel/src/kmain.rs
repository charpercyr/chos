use alloc::boxed::Box;
use chos_config::arch::mm::virt;
use chos_lib::arch::intr::breakpoint;
use chos_lib::elf::Elf;
use core::fmt::Write;
use core::mem::MaybeUninit;

use chos_lib::arch::qemu::{exit_qemu, QemuStatus};
use chos_lib::arch::serial::Serial;
use chos_lib::boot::KernelMemInfo;
use chos_lib::log::{debug, Bytes, LogHandler, TermColorLogHandler};
use chos_lib::sync::{Barrier, SpinOnceCell, Spinlock};

use crate::arch::early::unmap_lower_memory;
use crate::arch::kmain::ArchKernelArgs;
use crate::arch::mm::virt::init_kernel_virt;
use crate::intr::init_interrupts;
use crate::sched::enter_schedule;
use crate::symbols::add_elf_symbols_to_tree;

#[derive(Debug)]
pub struct KernelArgs {
    pub kernel_elf: Box<[u8]>,
    pub initrd: Option<Box<[u8]>>,
    pub core_count: usize,
    pub mem_info: KernelMemInfo,
    pub arch: ArchKernelArgs,
}

struct Logger {
    serial: Spinlock<Serial>,
}

impl LogHandler for Logger {
    fn log(&self, args: core::fmt::Arguments<'_>, _: chos_lib::log::LogLevel) {
        let mut serial = self.serial.lock();
        write!(&mut *serial, "{}", args).unwrap();
    }
    unsafe fn log_unsafe(&self, args: core::fmt::Arguments<'_>, _: chos_lib::log::LogLevel) {
        let serial = &mut *self.serial.get_ptr();
        write!(&mut *serial, "{}", args).unwrap();
    }
}

pub macro barrier($n:expr) {{
    static BARRIER: SpinOnceCell<Barrier> = SpinOnceCell::new();
    BARRIER.get_or_init(|| Barrier::new($n)).wait();
}}

fn setup_logger() {
    static mut LOGGER: MaybeUninit<TermColorLogHandler<Logger>> = MaybeUninit::uninit();
    unsafe {
        LOGGER = MaybeUninit::new(TermColorLogHandler::new(Logger {
            serial: Spinlock::new({
                let serial = Serial::new(0x3f8);
                serial.init()
            }),
        }));
        chos_lib::log::set_handler(LOGGER.assume_init_mut())
    }
}

pub fn kernel_main(id: usize, args: *const KernelArgs) -> ! {
    let args = unsafe { core::ptr::read(args) };

    if id == 0 {
        setup_logger();
        let kernel_elf = Elf::new(&args.kernel_elf).expect("Should be a valid elf");
        add_elf_symbols_to_tree(virt::STATIC_BASE, &kernel_elf);
    }

    barrier!(args.core_count);

    unsafe { init_interrupts() };

    // ANYTHING THAT NEEDS TO ACCESS EARLY MEMORY NEEDS TO BE DONE BEFORE THIS POINT

    unsafe { init_kernel_virt() };

    breakpoint();

    if id == 0 {
        unsafe { unmap_lower_memory(args.mem_info.total_size) };
        debug!(
            "[{}] Kernel len={}",
            id,
            Bytes(args.kernel_elf.len() as u64)
        );
        if let Some(i) = args.initrd.as_deref() {
            debug!("[{}] Initrd len={}", id, Bytes(i.len() as u64));
        }
        exit_qemu(QemuStatus::Success)
    }
    barrier!(args.core_count);
    enter_schedule()
}
