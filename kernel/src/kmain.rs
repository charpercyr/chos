use alloc::boxed::Box;
use alloc::string::String;
use core::fmt::Write;
use core::mem::MaybeUninit;

use chos_config::arch::mm::virt;
use chos_lib::arch::qemu::{exit_qemu, QemuStatus};
use chos_lib::arch::serial::Serial;
use chos_lib::boot::KernelMemInfo;
use chos_lib::elf::Elf;
use chos_lib::fmt::Bytes;
use chos_lib::log::{info, LogHandler, TermColorLogHandler, debug};
use chos_lib::sync::Spinlock;

use crate::arch::early::{init_non_early_memory, unmap_early_lower_memory};
use crate::arch::kmain::ArchKernelArgs;
use crate::arch::mm::virt::init_kernel_virt;
use crate::intr::init_interrupts;
use crate::sched::enter_schedule;
use crate::symbols::add_elf_symbols;
use crate::util::barrier;

#[derive(Debug)]
pub struct KernelArgs {
    pub kernel_elf: Box<[u8]>,
    pub initrd: Option<Box<[u8]>>,
    pub command_line: Option<String>,
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

fn setup_logger() {
    static mut LOGGER: MaybeUninit<TermColorLogHandler<Logger>> = MaybeUninit::uninit();
    unsafe {
        LOGGER = MaybeUninit::new(TermColorLogHandler::new(Logger {
            serial: Spinlock::new(Serial::com1().defaults()),
        }));
        chos_lib::log::set_handler(LOGGER.assume_init_mut())
    }
}

pub fn kernel_main(id: usize, args: &KernelArgs) -> ! {
    barrier!(args.core_count);

    if id == 0 {
        setup_logger();
    }

    // ANYTHING THAT NEEDS TO ACCESS EARLY MEMORY NEEDS TO BE DONE BEFORE THIS POINT
    // IF IT NEEDS TO HAPPEN ON OTHER CORES THAN ID 0, DON'T FORGET TO USE A BARRIER

    if id == 0 {
        unsafe {
            unmap_early_lower_memory(args.mem_info.total_size);
            init_non_early_memory(args);
        }

        add_elf_symbols(
            virt::STATIC_BASE,
            &Elf::new(&args.kernel_elf).expect("Should be a valid elf"),
        );
        
        debug!();
        debug!("##############");
        debug!("### KERNEL ###");
        debug!("##############");
        debug!();
    }

    barrier!(args.core_count);

    unsafe { init_interrupts() };
    unsafe { init_kernel_virt() };

    if id == 0 {
        info!(
            "Kernel len={} ({})",
            Bytes(args.kernel_elf.len() as u64),
            args.kernel_elf.len()
        );
        if let Some(i) = args.initrd.as_deref() {
            info!("Initrd len={}", Bytes(i.len() as u64));
        }
        exit_qemu(QemuStatus::Success)
    }
    barrier!(args.core_count);
    enter_schedule()
}
