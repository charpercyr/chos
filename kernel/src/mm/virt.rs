use bitflags::bitflags;
use chos_lib::init::ConstInit;
use chos_lib::mm::MapFlags;
use chos_lib::pool::{IArc, IArcAdapter, IArcCount};
use intrusive_collections::{linked_list, LinkedList};

use super::phys::MMPoolObjectAllocator;
use crate::arch::mm::virt::{ArchVMArea, ArchVMMap};

bitflags! {
    pub struct VMAreaFlags : u32 {
        const READ =        0b0000_0001;
        const WRITE =       0b0000_0010;
        const EXEC =        0b0000_0100;
        const SHARED =      0b0000_1000;

        const USER =        0b0001_0000;
    }
}

pub struct VMMap {
    areas: LinkedList<VMAreaAdapter>,
    arch: ArchVMMap,
}

pub struct VMArea {
    count: IArcCount,
    link: linked_list::AtomicLink,
    pub flags: MapFlags,
    pub arch: ArchVMArea,
}

impl IArcAdapter for VMArea {
    fn count(&self) -> &IArcCount {
        &self.count
    }
}

chos_lib::intrusive_adapter!(VMAreaAdapter = VMAreaArc : VMArea  { link: linked_list::AtomicLink });

const VM_AREA_SLAB_ORDER: u8 = 0;
static VMAREA_SLAB_POOL: MMPoolObjectAllocator<VMArea, VM_AREA_SLAB_ORDER> =
    MMPoolObjectAllocator::INIT;
chos_lib::pool!(pub struct VMAreaPool: VMArea => &VMAREA_SLAB_POOL);

pub type VMAreaArc = IArc<VMArea, VMAreaPool>;

impl VMMap {
    pub const fn new() -> Self {
        Self {
            areas: LinkedList::new(VMAreaAdapter::new()),
            arch: ArchVMMap::INIT,
        }
    }
}

impl ConstInit for VMMap {
    const INIT: Self = Self::new();
}
