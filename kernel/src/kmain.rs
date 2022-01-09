use alloc::boxed::Box;

use chos_lib::{arch::qemu::{exit_qemu, QemuStatus}, log::{println, Bytes}};

#[derive(Debug)]
pub struct KernelArgs {
    pub kernel_elf: Option<Box<[u8]>>,
    pub initrd: Option<Box<[u8]>>,
}

#[no_mangle]
extern "C" fn kernel_main(id: usize, args: *const KernelArgs) -> ! {
    let args = unsafe { &*args };
    if let Some(kernel_elf) = &args.kernel_elf {
        println!("[{}] Kernel ELF @ {:p} len = {}", id, &*kernel_elf, Bytes(kernel_elf.len() as u64))
    } else {
        println!("[{}] No kernel", id)
    }
    if let Some(initrd) = &args.initrd {
        println!("[{}] InitRd ELF @ {:p} len = {}", id, &*initrd, Bytes(initrd.len() as u64))
    } else {
        println!("[{}] No InitRd", id)
    }
    if id == 0 {
        unsafe { drop(Box::from_raw(args as *const KernelArgs as *mut KernelArgs)) }
    }
    loop {}
    // exit_qemu(QemuStatus::Success);
}
