pub mod stack;

use core::mem::MaybeUninit;

use chos_config::arch::mm::{phys, virt};
use chos_lib::mm::{PAddr, PFrame, VAddr, VFrame, VFrameRange};

use self::stack::init_kernel_stacks;
use super::phys::Page;
use crate::kmain::KernelArgs;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MemoryRegionType {
    Alloc,
    Normal,
    PerCpu,
    IoMem,
    Static,
    Stack,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MemoryMapError {
    CannotMap,
    RegionNotFound,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PageFaultResult {
    NotMapped,
    Mapped(PAddr),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PageFaultReason {
    Read,
    Write,
}

trait MemoryRegion {
    fn typ(&self) -> MemoryRegionType;
    fn name(&self) -> &str;

    fn vaddr_range(&self) -> VFrameRange;
    fn paddr_of(&self, vaddr: VAddr) -> Option<PAddr>;
    fn map_paddr(&self, pframe: PFrame) -> Result<VFrame, MemoryMapError>;

    fn handle_page_fault(&self, reason: PageFaultReason) -> PageFaultResult;
}

struct IdentityMemoryRegion {
    name: &'static str,
    base: VFrame,
    size: u64,
    typ: MemoryRegionType,
}

impl MemoryRegion for IdentityMemoryRegion {
    fn typ(&self) -> MemoryRegionType {
        self.typ
    }
    fn name(&self) -> &str {
        self.name
    }

    fn vaddr_range(&self) -> VFrameRange {
        VFrameRange::new(self.base, self.base.add(virt::MEMORY_ZONE_FRAMES))
    }

    fn paddr_of(&self, vaddr: VAddr) -> Option<PAddr> {
        if self.vaddr_range().contains_address(vaddr) {
            Some(PAddr::new((vaddr - self.base.addr()).as_u64()))
        } else {
            None
        }
    }
    fn map_paddr(&self, pframe: PFrame) -> Result<VFrame, MemoryMapError> {
        Ok(self.base + pframe)
    }

    fn handle_page_fault(&self, _: PageFaultReason) -> PageFaultResult {
        PageFaultResult::NotMapped
    }
}

struct StaticMemoryRegion {
    pbase: PFrame,
    vbase: VFrame,
    size: u64,
}

impl MemoryRegion for StaticMemoryRegion {
    fn typ(&self) -> MemoryRegionType {
        MemoryRegionType::Static
    }
    fn name(&self) -> &str {
        "kernel"
    }

    fn vaddr_range(&self) -> VFrameRange {
        VFrameRange::new(self.vbase, self.vbase.add(virt::MEMORY_ZONE_FRAMES))
    }

    fn paddr_of(&self, vaddr: VAddr) -> Option<PAddr> {
        Some(PAddr::new((vaddr - self.vbase.addr()).as_u64()) + self.pbase.addr())
    }

    fn map_paddr(&self, _: PFrame) -> Result<VFrame, MemoryMapError> {
        Err(MemoryMapError::CannotMap)
    }

    fn handle_page_fault(&self, _: PageFaultReason) -> PageFaultResult {
        PageFaultResult::NotMapped
    }
}

pub struct StackMemoryRegion;

impl MemoryRegion for StackMemoryRegion {
    fn typ(&self) -> MemoryRegionType {
        MemoryRegionType::Stack
    }
    fn name(&self) -> &str {
        "kernel"
    }

    fn vaddr_range(&self) -> VFrameRange {
        todo!()
    }

    fn paddr_of(&self, vaddr: VAddr) -> Option<PAddr> {
        todo!()
    }

    fn map_paddr(&self, _: PFrame) -> Result<VFrame, MemoryMapError> {
        Err(MemoryMapError::CannotMap)
    }

    fn handle_page_fault(&self, reason: PageFaultReason) -> PageFaultResult {
        todo!()
    }
}

static mut ALLOC_REGION: IdentityMemoryRegion = IdentityMemoryRegion {
    typ: MemoryRegionType::Alloc,
    name: "alloc",
    base: virt::PHYSICAL_MAP_BASE,
    size: 0,
};
static mut IOMEM_REGION: IdentityMemoryRegion = IdentityMemoryRegion {
    typ: MemoryRegionType::IoMem,
    name: "iomem",
    base: virt::DEVICE_BASE,
    size: 0,
};
static mut HEAP_REGION: IdentityMemoryRegion = IdentityMemoryRegion {
    typ: MemoryRegionType::Normal,
    name: "heap",
    base: virt::HEAP_BASE,
    size: 0,
};
static mut STATIC_REGION: StaticMemoryRegion = StaticMemoryRegion {
    pbase: phys::KERNEL_DATA_BASE,
    vbase: virt::STATIC_BASE,
    size: 0,
};
static mut ALL_MEMORY_REGIONS: MaybeUninit<[&'static (dyn MemoryRegion + Sync); 5]> = unsafe {
    MaybeUninit::new([
        &ALLOC_REGION,
        &HEAP_REGION,
        &IOMEM_REGION,
        &STATIC_REGION,
        &StackMemoryRegion,
    ])
};

fn get_memory_region_by_type(typ: MemoryRegionType) -> Option<&'static (dyn MemoryRegion + Sync)> {
    for &region in unsafe { ALL_MEMORY_REGIONS.assume_init_ref() } {
        if region.typ() == typ {
            return Some(region);
        }
    }
    None
}

fn get_memory_region_by_vaddr(vaddr: VAddr) -> Option<&'static (dyn MemoryRegion + Sync)> {
    for &region in unsafe { ALL_MEMORY_REGIONS.assume_init_ref() } {
        if region.vaddr_range().contains_address(vaddr) {
            return Some(region);
        }
    }
    None
}

pub unsafe fn map_paddr(pframe: PFrame, typ: MemoryRegionType) -> Result<VFrame, MemoryMapError> {
    get_memory_region_by_type(typ)
        .ok_or(MemoryMapError::RegionNotFound)
        .and_then(|r| r.map_paddr(pframe))
}

pub unsafe fn map_page(page: &Page, typ: MemoryRegionType) -> Result<VFrame, MemoryMapError> {
    map_paddr(page.frame, typ)
}

pub fn paddr_of(vaddr: VAddr, typ: MemoryRegionType) -> Option<PAddr> {
    get_memory_region_by_type(typ).and_then(|r| r.paddr_of(vaddr))
}

pub unsafe fn unmap_page(_: VAddr) {
    // Nothing
}

pub fn handle_kernel_page_fault(vaddr: VAddr, reason: PageFaultReason) -> PageFaultResult {
    get_memory_region_by_vaddr(vaddr)
        .map(|r| r.handle_page_fault(reason))
        .unwrap_or(PageFaultResult::NotMapped)
}

pub unsafe fn init_kernel_virt(args: &KernelArgs) {
    init_kernel_stacks(args.core_count, &[args.early_stacks]);
    ALLOC_REGION.size = args.mem_info.total_size;
    HEAP_REGION.size = args.mem_info.total_size;
    IOMEM_REGION.size = args.mem_info.total_size;
    STATIC_REGION.size = args.mem_info.code.size as u64;
    ALL_MEMORY_REGIONS = MaybeUninit::new([
        &ALLOC_REGION,
        &HEAP_REGION,
        &IOMEM_REGION,
        &STATIC_REGION,
        &StackMemoryRegion,
    ]);
}
