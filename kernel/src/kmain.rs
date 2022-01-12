use alloc::boxed::Box;

use chos_lib::arch::hlt_loop;
use chos_lib::arch::qemu::{exit_qemu, QemuStatus};
use chos_lib::log::{println, Bytes};
use chos_lib::sync::{Barrier, SpinOnceCell};

#[derive(Debug)]
pub struct KernelArgs {
    pub kernel_elf: Option<Box<[u8]>>,
    pub initrd: Option<Box<[u8]>>,
    pub core_count: usize,
}

#[no_mangle]
extern "C" fn kernel_main(id: usize, args: &KernelArgs) -> ! {
    if let Some(kernel_elf) = args.kernel_elf.as_deref() {
        println!(
            "[{}] Kernel ELF @ {:p} len = {} ({})",
            id,
            kernel_elf,
            Bytes(kernel_elf.len() as u64),
            kernel_elf.len(),
        );
    } else {
        println!("[{}] No kernel", id);
    }
    if let Some(initrd) = args.initrd.as_deref() {
        println!(
            "[{}] InitRd ELF @ {:p} len = {}",
            id,
            initrd,
            Bytes(initrd.len() as u64)
        );
    } else {
        println!("[{}] No InitRd", id);
    }
    static B: SpinOnceCell<Barrier> = SpinOnceCell::new();
    B.get_or_init(|| Barrier::new(args.core_count)).wait();
    if id != 0 {
        hlt_loop()
    } else {
        unsafe { drop(Box::from_raw(args as *const KernelArgs as *mut KernelArgs)) }
        exit_qemu(QemuStatus::Success)
    }
}
