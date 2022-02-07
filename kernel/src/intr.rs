use crate::arch::intr::{arch_init_interrupts, arch_init_interrupts_cpu};
use crate::kmain::KernelArgs;

pub unsafe fn init_interrupts(args: &KernelArgs) {
    arch_init_interrupts(args);
}

pub unsafe fn init_interrupts_cpu(args: &KernelArgs) {
    arch_init_interrupts_cpu(args);
}
