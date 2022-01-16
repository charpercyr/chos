use crate::arch::intr::arch_init_interrupts;


pub unsafe fn init_interrupts() {
    arch_init_interrupts();
}