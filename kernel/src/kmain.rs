use alloc::boxed::Box;
use alloc::string::String;
use chos_lib::Volatile;
use core::mem::MaybeUninit;

use chos_config::arch::mm::virt;
use chos_lib::arch::serial::Serial;
use chos_lib::boot::KernelMemInfo;
use chos_lib::elf::Elf;
use chos_lib::log::{debug, println, LogHandler, TermColorLogHandler};
use chos_lib::mm::VAddr;
use chos_lib::sync::{SpinLazy, Spinlock};

use crate::arch::asm::call_with_stack;
use crate::arch::early::{init_non_early_memory, unmap_early_lower_memory};
use crate::arch::kmain::ArchKernelArgs;
use crate::arch::mm::virt::init_kernel_virt;
use crate::cpumask::init_cpumask;
use crate::early::EarlyStacks;
use crate::intr::{init_interrupts, init_interrupts_cpu};
use crate::mm::this_cpu_info;
use crate::mm::virt::stack::{alloc_kernel_stack, Stack};
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
    pub early_stacks: EarlyStacks,
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

unsafe fn do_enter_schedule(stack: VAddr) -> ! {
    extern "C" fn call_schedule(_: u64, _: u64, _: u64, _: u64) -> ! {
        enter_schedule();
    }
    call_with_stack(call_schedule, stack, 0, 0, 0, 0)
}

pub fn kernel_main(id: usize, args: &KernelArgs) -> ! {
    {
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

        barrier!(args.core_count);

        {
            static STACK: SpinLazy<Stack> = SpinLazy::new(|| alloc_kernel_stack(0).unwrap());
            if id == 0 {
                let stack = STACK.get();
                let ptr: &mut Volatile<u32> = unsafe { stack.range.start().addr().as_mut() };
                println!("Writing @ {:#p}", ptr);
                ptr.write(0xdeadbeef);
            }
            barrier!(args.core_count);
            if id == 1 {
                let stack = STACK.get();
                let ptr: &Volatile<u32> = unsafe { stack.range.start().addr().as_ref() };
                println!("Got {:#x} @ {:#p}", ptr.read(), ptr);
            }
        }
    }

    let (base, size) = args.early_stacks.get_for(id);
    let stack = base + size;

    unsafe { do_enter_schedule(stack) }
}
