use core::arch::asm;
use core::mem::MaybeUninit;

use chos_config::arch::mm::{stack, virt};
use chos_lib::arch::mm::FrameSize4K;
use chos_lib::boot::KernelBootInfo;
use chos_lib::check_kernel_entry;
use chos_lib::log::*;
use chos_lib::mm::{VAddr, VFrame};

use crate::arch::asm::call_with_stack;
use crate::arch::early::{arch_copy_boot_data, arch_init_early_memory, use_early_kernel_table};
use crate::arch::mm::per_cpu::init_per_cpu_data_for_cpu;
use crate::kmain::{kernel_main, KernelArgs};
use crate::mm::virt::stack::{alloc_early_stack, Stack};
use crate::util::barrier;

fn hlt_loop() -> ! {
    unsafe {
        asm! {
            "cli",
            "0: hlt",
            "jmp 0b",
            options(nomem, nostack, att_syntax, noreturn),
        }
    }
}

pub unsafe fn init_early_memory(info: &KernelBootInfo) {
    arch_init_early_memory(info)
}

pub unsafe fn init_early_memory_secondary(id: usize) {
    use_early_kernel_table();
    init_per_cpu_data_for_cpu(id);
}

const MAX_CPUS: usize = 16;
struct EarlyData {
    stacks: &'static [MaybeUninit<Stack>; MAX_CPUS],
    kernel_args: KernelArgs,
}

unsafe fn populate_kernel_args(info: &KernelBootInfo, stacks: &'static [Stack]) -> KernelArgs {
    KernelArgs {
        kernel_elf: info.elf.as_ref().into(),
        initrd: info.initrd.map(|ird| ird.as_ref().into()),
        core_count: info.core_count,
        mem_info: info.mem_info,
        command_line: info.command_line.map(Into::into),
        arch: arch_copy_boot_data(&info.arch),
        early_stacks: stacks,
    }
}

unsafe fn enter_kernel_main(id: usize, args: &KernelArgs, stack: VAddr) -> ! {
    extern "C" fn call_kernel_main(id: u64, args: u64, _: u64, _: u64) -> ! {
        kernel_main(id as usize, unsafe { &*(args as *const KernelArgs) })
    }
    call_with_stack(
        call_kernel_main,
        stack,
        id as u64,
        args as *const KernelArgs as u64,
        0,
        0,
    )
}

static mut STACKS_BASE: VFrame<FrameSize4K> = virt::STACK_BASE;

#[no_mangle]
pub fn entry(info: &KernelBootInfo, id: usize) -> ! {
    static mut EARLY_DATA: MaybeUninit<EarlyData> = MaybeUninit::uninit();

    if id == 0 {
        unsafe { chos_lib::log::set_handler(info.early_log) };

        debug!();
        debug!("####################");
        debug!("### EARLY KERNEL ###");
        debug!("####################");
        debug!();

        unsafe {
            init_early_memory(info);
            static mut STACKS: [MaybeUninit<Stack>; MAX_CPUS] = [MaybeUninit::uninit(); MAX_CPUS];
            for i in 0..info.core_count {
                STACKS[i] =
                    MaybeUninit::new(alloc_early_stack(stack::KERNEL_STACK_PAGE_ORDER).unwrap());
            }
            EARLY_DATA = MaybeUninit::new(EarlyData {
                stacks: &STACKS,
                kernel_args: populate_kernel_args(
                    info,
                    MaybeUninit::slice_assume_init_ref(&STACKS[0..info.core_count]),
                ),
            });
        }
    }

    barrier!(info.core_count);

    if id != 0 {
        unsafe { init_early_memory_secondary(id) };
    }

    unsafe {
        let EarlyData {
            stacks,
            kernel_args,
        } = EARLY_DATA.assume_init_ref();
        let stack_base = stacks[id].assume_init().range.end().addr();
        debug!("[{}] Using stack @ {:x}", id, stack_base);
        enter_kernel_main(id, kernel_args, stack_base);
    }
}
check_kernel_entry!(entry);
