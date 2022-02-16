use chos_lib::arch::intr::hlt;

pub fn sched_idle() {
    hlt()
}