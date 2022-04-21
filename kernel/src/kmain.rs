use alloc::boxed::Box;
use alloc::format;
use alloc::string::String;
use alloc::sync::Arc;
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
use crate::async_::AsyncSem;
use crate::cpumask::init_cpumask;
use crate::initrd::load_initrd;
use crate::intr::{init_interrupts, init_interrupts_cpu};
use crate::mm::this_cpu_info;
use crate::mm::virt::stack::Stack;
use crate::module::{get_modules_for_elf, Module};
use crate::sched::enter_schedule;
use crate::sched::ktask::{init_ktask_stack, spawn, spawn_future};
use crate::symbols::add_elf_symbols;
use crate::timer::init_timer;
use crate::util::{barrier, do_once};

#[derive(Debug)]
pub struct KernelArgs {
    pub kernel_elf: Box<[u8]>,
    pub initrd: Box<[u8]>,
    pub command_line: Option<String>,
    pub core_count: usize,
    pub mem_info: KernelMemInfo,
    pub arch: ArchKernelArgs,
    pub early_stacks: &'static [Stack],
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
        assert_eq!(args.early_stacks.len(), args.core_count);
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
            virt::STATIC_BASE.addr(),
            &Elf::new(&args.kernel_elf).expect("Should be a valid elf"),
        );
    }

    barrier!(args.core_count);

    // ANYTHING THAT NEEDS TO ACCESS LOWER MEMORY NEEDS TO BE DONE BEFORE THIS POINT

    unsafe { init_kernel_virt() };

    if id == 0 {
        unsafe { init_interrupts(args) };
    }
    barrier!(args.core_count);
    unsafe { init_interrupts_cpu(args) };

    if id == 0 {
        init_timer(args);
    }

    init_ktask_stack(args.early_stacks[id]);

    do_once!({
        let mods = get_modules_for_elf(
            &Elf::new(&args.kernel_elf).unwrap(),
            virt::STATIC_BASE.addr(),
        )
        .expect("Static modules are invalid");
        let mods_count = mods.len();
        let sem = Arc::new(AsyncSem::zero());
        for m in mods {
            let module = Module { decl: m };
            if let Some(init) = m.init() {
                let sem = sem.clone();
                spawn(
                    move || {
                        init(module);
                        sem.signal();
                    },
                    format!("[mod-init:{}]", m.name()),
                );
            }
        }
        let initrd = args.initrd.clone();
        spawn_future(
            async move {
                sem.wait_count(mods_count).await;
                load_initrd(&initrd).await;
            },
            "[initrd]",
        );
    });

    enter_schedule();
}
