pub mod phys;
pub mod slab;

mod global;

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
