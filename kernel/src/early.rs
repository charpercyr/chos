use core::arch::asm;
use core::mem::MaybeUninit;

use chos_lib::arch::mm::VAddr;
use chos_lib::boot::KernelBootInfo;
use chos_lib::check_kernel_entry;
use chos_lib::log::*;
use chos_lib::sync::{SpinBarrier, SpinOnceCell};

use crate::arch::asm::call_with_stack;
use crate::arch::early::{arch_copy_boot_data, arch_init_early_memory, use_early_kernel_table};
use crate::arch::mm::per_cpu::init_per_cpu_data_for_cpu;
use crate::kmain::{kernel_main, KernelArgs};
use crate::mm::stack::Stacks;
use crate::mm::stack::allocate_kernel_stacks;

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

struct EarlyData {
    stacks: Stacks,
    kernel_args: KernelArgs,
}

unsafe fn copy_boot_data(info: &KernelBootInfo, stacks: Stacks) -> KernelArgs {
    KernelArgs {
        kernel_elf: info.elf.as_ref().into(),
        initrd: info.initrd.map(|ird| ird.as_ref().into()),
        core_count: info.core_count,
        mem_info: info.mem_info,
        command_line: info.command_line.map(Into::into),
        arch: arch_copy_boot_data(&info.arch),
        stacks,
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

#[no_mangle]
pub fn entry(info: &KernelBootInfo, id: usize) -> ! {
    static BARRIER: SpinOnceCell<SpinBarrier> = SpinOnceCell::new();
    static mut EARLY_DATA: MaybeUninit<EarlyData> = MaybeUninit::uninit();
    let barrier = BARRIER.get_or_init(|| SpinBarrier::new(info.core_count));

    if id == 0 {
        unsafe { chos_lib::log::set_handler(info.early_log) };

        debug!();
        debug!("####################");
        debug!("### EARLY KERNEL ###");
        debug!("####################");
        debug!();

        unsafe {
            init_early_memory(info);
            let stacks = allocate_kernel_stacks(info.core_count);
            EARLY_DATA = MaybeUninit::new(EarlyData {
                stacks,
                kernel_args: copy_boot_data(info, stacks),
            });
        }
    }

    barrier.wait();

    if id != 0 {
        unsafe { init_early_memory_secondary(id) };
    }

    unsafe {
        let EarlyData {
            stacks,
            kernel_args,
        } = EARLY_DATA.assume_init_ref();
        let stack_base = stacks.base + stacks.stride * id as u64 + stacks.size;
        debug!("[{}] Using stack @ {:x}", id, stack_base);
        enter_kernel_main(id, kernel_args, stack_base);
    }
}
check_kernel_entry!(entry);
