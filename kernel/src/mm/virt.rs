use chos_lib::init::ConstInit;
use chos_lib::intrusive::list;
use chos_lib::mm::MapFlags;
use chos_lib::pool::{IArc, IArcAdapter, IArcCount};

use super::phys::MMPoolObjectAllocator;
use crate::arch::mm::virt::{ArchVMArea, ArchVMMap};

pub struct VMMap {
    areas: list::HList<VMAreaAdapter>,
    arch: ArchVMMap,
}

pub struct VMArea {
    count: IArcCount,
    link: list::AtomicLink<VMAreaPool>,
    pub flags: MapFlags,
    arch: ArchVMArea,
}

impl IArcAdapter for VMArea {
    fn count(&self) -> &IArcCount {
        &self.count
    }
}
chos_lib::intrusive_adapter!(struct VMAreaAdapter = VMAreaArc : VMArea { link: list::AtomicLink<VMAreaPool>});

const VM_AREA_SLAB_ORDER: u8 = 0;
static VMAREA_SLAB_POOL: MMPoolObjectAllocator<VMArea, VM_AREA_SLAB_ORDER> =
    MMPoolObjectAllocator::INIT;
chos_lib::pool!(pub struct VMAreaPool: VMArea => &VMAREA_SLAB_POOL);

pub type VMAreaArc = IArc<VMArea, VMAreaPool>;

impl VMMap {
    pub const fn new() -> Self {
        Self {
            areas: ConstInit::INIT,
            arch: ArchVMMap::INIT,
        }
    }
}

impl ConstInit for VMMap {
    const INIT: Self = Self::new();
}

