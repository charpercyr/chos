use alloc::boxed::Box;
use alloc::string::String;
use core::mem::MaybeUninit;

use chos_config::arch::mm::virt;
use chos_lib::arch::serial::Serial;
use chos_lib::boot::KernelMemInfo;
use chos_lib::elf::Elf;
use chos_lib::log::{debug, LogHandler, TermColorLogHandler};
use chos_lib::sync::Spinlock;

use crate::arch::early::{init_non_early_memory, unmap_early_lower_memory};
use crate::arch::kmain::ArchKernelArgs;
use crate::arch::mm::virt::init_kernel_virt;
use crate::cpumask::init_cpumask;
use crate::intr::{init_interrupts, init_interrupts_cpu};
use crate::mm::this_cpu_info;
use crate::sched::enter_schedule;
use crate::symbols::add_elf_symbols;
use crate::timer::init_timer;
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
    serial: TermColorLogHandler<Spinlock<Serial>>,
}

impl LogHandler for Logger {
    fn log(&self, args: core::fmt::Arguments<'_>, lvl: chos_lib::log::LogLevel) {
        self.serial
            .log(format_args!("[{}] {}", this_cpu_info().id, args), lvl)
    }
    unsafe fn log_unsafe(&self, args: core::fmt::Arguments<'_>, lvl: chos_lib::log::LogLevel) {
        self.serial
            .log_unsafe(format_args!("[{}] {}", this_cpu_info().id, args), lvl)
    }
}

fn setup_logger() {
    static mut LOGGER: MaybeUninit<Logger> = MaybeUninit::uninit();
    unsafe {
        LOGGER = MaybeUninit::new(Logger {
            serial: TermColorLogHandler::new(Spinlock::new(Serial::com1().defaults())),
        });
        chos_lib::log::set_handler(LOGGER.assume_init_mut())
    }
}

pub fn kernel_main(id: usize, args: &KernelArgs) -> ! {
    barrier!(args.core_count);

    if id == 0 {
        setup_logger();
        init_cpumask(args.core_count);

        debug!();
        debug!("##############");
        debug!("### KERNEL ###");
        debug!("##############");
        debug!();
    }

    // ANYTHING THAT NEEDS TO ACCESS LOWER HALF MEMORY NEEDS TO BE DONE BEFORE THIS POINT

    if id == 0 {
        unsafe {
            unmap_early_lower_memory(args.mem_info.total_size);
            init_non_early_memory(args);
        }

        add_elf_symbols(
            virt::STATIC_BASE,
            &Elf::new(&args.kernel_elf).expect("Should be a valid elf"),
        );

        unsafe { init_interrupts(args) };
    }

    barrier!(args.core_count);

    unsafe { init_interrupts_cpu(args) };

    // ANYTHING THAT NEEDS TO ACCESS LOWER MEMORY NEEDS TO BE DONE BEFORE THIS POINT

    unsafe { init_kernel_virt() };

    if id == 0 {
        init_timer(args);
    }

    barrier!(args.core_count);
    enter_schedule()
}
