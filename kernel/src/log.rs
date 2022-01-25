
use crate::mm::per_cpu;

const PER_CPU_RING_SIZE: usize = 1 * 1024 * 1024;

per_cpu! {
    static mut ref RING_BUFFER: [u8; PER_CPU_RING_SIZE] = [0; PER_CPU_RING_SIZE];
}
