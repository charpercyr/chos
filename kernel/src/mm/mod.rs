pub mod phys;
pub mod slab;

mod global;

pub use crate::arch::mm::*;

pub enum VirtualMemoryZone {
    Physical,
    Paging,
    Device,
    Static,
    Heap,
    PerCpuStatic,
    PerCpuHeap,
    Stack,
}
