
use core::fmt::Arguments;

use chos_lib::fmt::size_of_fmt;
use crate::mm::PerCpu;

use crate::mm::per_cpu;

const PER_CPU_RING_SIZE: usize = 1 * 1024 * 1024;

struct LogBuffer {
    buf: [u8; PER_CPU_RING_SIZE],
}
impl LogBuffer {
    pub const fn new() -> Self {
        Self {
            buf: [0; PER_CPU_RING_SIZE],
        }
    }

    pub fn write_fmt(&self, fmt: Arguments<'_>) {
        let size = size_of_fmt(fmt);
        let size_bytes = size.to_ne_bytes();
        drop(size_bytes);
        todo!()
    }
}

per_cpu! {
    static mut ref RING_BUFFER: LogBuffer = LogBuffer::new();
}

fn log(fmt: Arguments<'_>) {
    RING_BUFFER.with(move |rb| rb.write_fmt(fmt));
}
