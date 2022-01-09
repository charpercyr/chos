use core::arch::asm;
use core::mem::MaybeUninit;

use alloc::borrow::ToOwned;
use alloc::boxed::Box;
use chos_lib::arch::mm::VAddr;
use chos_lib::boot::KernelBootInfo;
use chos_lib::check_kernel_entry;
use chos_lib::log::*;
use chos_lib::sync::{Barrier, SpinOnceCell};

use crate::arch::early::arch_init_early_memory;
use crate::arch::early::use_early_kernel_table;
use crate::arch::mm::per_cpu::init_per_cpu_data_for_cpu;
use crate::early::stack::Stacks;
use crate::kmain::KernelArgs;

mod stack {
    use chos_config::arch::mm::{stack, virt};
    use chos_lib::arch::mm::{VAddr, PAGE_SIZE64};
    use raw_alloc::AllocFlags;

    use crate::arch::early::map_stack;
    use crate::mm::phys::raw_alloc;

    #[derive(Clone, Copy, Debug)]
    pub struct Stacks {
        pub base: VAddr,
        pub size: u64,
        pub stride: u64,
    }

    static mut STACKS_BASE: VAddr = virt::STACK_BASE;

    unsafe fn allocate_stack(order: u8) -> VAddr {
        let pages = raw_alloc::alloc_pages(order, AllocFlags::empty()).expect("Should not fail");

        map_stack(pages, 1 << order, true)
    }

    pub unsafe fn allocate_stacks(stack_count: usize) -> Stacks {
        let base = STACKS_BASE;
        let stride = (PAGE_SIZE64 << stack::KERNEL_STACK_PAGE_ORDER) + PAGE_SIZE64;

        for _ in 0..stack_count {
            allocate_stack(stack::KERNEL_STACK_PAGE_ORDER);
        }

        Stacks {
            base,
            size: PAGE_SIZE64 << stack::KERNEL_STACK_PAGE_ORDER,
            stride,
        }
    }
}

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

extern "C" {
    fn enter_kernel_main(id: usize, args: *const KernelArgs, stack: VAddr) -> !;
}

pub unsafe fn init_early_memory(info: &KernelBootInfo) {
    arch_init_early_memory(info)
}

pub unsafe fn init_early_memory_secondary(id: usize) {
    use_early_kernel_table();
    init_per_cpu_data_for_cpu(id);
}

#[derive(Clone, Copy)]
struct EarlyData {
    stacks: Stacks,
    kernel_args: *const KernelArgs,
}

#[no_mangle]
pub fn entry(info: &KernelBootInfo, id: usize) -> ! {
    static BARRIER: SpinOnceCell<Barrier> = SpinOnceCell::new();
    static mut EARLY_DATA: MaybeUninit<EarlyData> = MaybeUninit::uninit();
    let barrier = BARRIER.get_or_init(|| Barrier::new(info.core_count));

    if id == 0 {
        unsafe { chos_lib::log::set_handler(info.early_log) };

        debug!("####################");
        debug!("### EARLY KERNEL ###");
        debug!("####################");
    
        unsafe {
            init_early_memory(info);
            let stacks = stack::allocate_stacks(info.core_count);
            EARLY_DATA = MaybeUninit::new(EarlyData {
                stacks,
                kernel_args: Box::leak(Box::new(KernelArgs {
                    kernel_elf: (info.elf.is_null()).then(|| (&*info.elf).to_owned().into()),
                    initrd: (info.initrd.is_null()).then(|| (&*info.initrd).to_owned().into()),
                })),
            });
        }
    }

    // TODO Copy kernel info to heap & unmap lower half

    barrier.wait();
    
    if id != 0 {
        unsafe { init_early_memory_secondary(id) };
    }

    unsafe {
        let EarlyData { stacks, kernel_args } = *EARLY_DATA.assume_init_ref();
        let stack_base = stacks.base + stacks.stride * id as u64 + stacks.size;
        debug!("[{}] Using stack @ {:x}", id, stack_base);
        enter_kernel_main(id, kernel_args, stack_base);
    }
}
check_kernel_entry!(entry);
