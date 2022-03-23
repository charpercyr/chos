use core::arch::asm;
use core::mem::MaybeUninit;

use chos_config::arch::mm::{stack, virt};
use chos_lib::arch::mm::FrameSize4K;
use chos_lib::boot::KernelBootInfo;
use chos_lib::check_kernel_entry;
use chos_lib::log::*;
use chos_lib::mm::FrameSize;
use chos_lib::mm::{VAddr, VFrame};
use chos_lib::sync::{SpinBarrier, SpinOnceCell};
use raw_alloc::AllocFlags;

use crate::arch::asm::call_with_stack;
use crate::arch::early::{
    arch_copy_boot_data, arch_init_early_memory, map_stack, use_early_kernel_table,
};
use crate::arch::mm::per_cpu::init_per_cpu_data_for_cpu;
use crate::kmain::{kernel_main, KernelArgs};
use crate::mm::phys::raw_alloc;
use crate::mm::this_cpu_info;

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
    stacks: EarlyStacks,
    kernel_args: KernelArgs,
}

unsafe fn copy_boot_data(info: &KernelBootInfo, stacks: EarlyStacks) -> KernelArgs {
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

const USE_STACK_GUARD_PAGE: bool = true;

#[derive(Clone, Copy, Debug)]
pub struct EarlyStacks {
    pub base: VAddr,
    pub size: u64,
    pub stride: u64,
}

impl EarlyStacks {
    pub fn get_for(&self, id: usize) -> (VAddr, u64) {
        (self.base + (id as u64) * self.stride, self.size)
    }

    pub fn get_for_this_cpu(&self) -> (VAddr, u64) {
        self.get_for(this_cpu_info().id)
    }
}

static mut STACKS_BASE: VFrame<FrameSize4K> = virt::STACK_BASE;

unsafe fn allocate_early_stack(order: u8) -> VAddr {
    let pages = raw_alloc::alloc_pages(order, AllocFlags::empty())
        .expect("Should not fail")
        .addr();
    let vaddr = STACKS_BASE;
    map_stack(vaddr, pages, 1 << order);
    STACKS_BASE = STACKS_BASE.add(1 << order);
    if USE_STACK_GUARD_PAGE {
        STACKS_BASE = STACKS_BASE.add(1);
    }
    vaddr.addr()
}

pub unsafe fn allocate_early_stacks_order(stack_count: usize, order: u8) -> EarlyStacks {
    let base = STACKS_BASE;
    let stride = (FrameSize4K::PAGE_SIZE << order) + FrameSize4K::PAGE_SIZE;

    for _ in 0..stack_count {
        allocate_early_stack(order);
    }

    EarlyStacks {
        base: base.addr(),
        size: FrameSize4K::PAGE_SIZE << order,
        stride,
    }
}

pub unsafe fn allocate_early_stacks(stack_count: usize) -> EarlyStacks {
    allocate_early_stacks_order(stack_count, stack::KERNEL_STACK_PAGE_ORDER)
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
            let stacks = allocate_early_stacks(info.core_count);
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
