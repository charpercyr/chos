#![no_std]

#![feature(asm)]
#![feature(associated_type_bounds)]

mod bt;
pub use bt::*;

pub mod apic;

pub fn rdtsc() -> u64 {
    let cl: u32;
    let cu: u32;
    unsafe {
        asm!(
            "rdtsc",
            out("eax") cl,
            out("edx") cu
        );
    }
    let cl = cl as u64;
    let cu = cu as u64;
    cu << 32 | cl
}

pub fn busy_loop(mut cycles: usize) {
    while cycles > 0 {
        unsafe { asm!("pause") };
        cycles -= 1;
    }
}