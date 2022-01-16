use alloc::boxed::Box;

use chos_lib::arch::qemu::{exit_qemu, QemuStatus};
use chos_lib::log::{debug, Bytes};
use chos_lib::sync::{Barrier, SpinOnceCell};

use crate::arch::kmain::ArchKernelArgs;
use crate::intr::init_interrupts;
use crate::sched::enter_schedule;

#[derive(Debug)]
pub struct KernelArgs {
    pub kernel_elf: Box<[u8]>,
    pub initrd: Option<Box<[u8]>>,
    pub core_count: usize,
    pub arch: ArchKernelArgs,
}

pub fn kernel_main(id: usize, args: &KernelArgs) -> ! {
    unsafe { init_interrupts() };
    if id == 0 {
        debug!("Kernel len={}", Bytes(args.kernel_elf.len() as u64));
        if let Some(i) = &args.initrd {
            debug!("Initrd len={}", Bytes(i.len() as u64));
        }
        exit_qemu(QemuStatus::Success)
    }
    static SCHED_BARRIER: SpinOnceCell<Barrier> = SpinOnceCell::new();
    SCHED_BARRIER
        .get_or_init(|| Barrier::new(args.core_count))
        .wait();
    enter_schedule()
}
