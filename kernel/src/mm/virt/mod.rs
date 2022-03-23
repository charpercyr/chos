pub mod stack;

use chos_config::arch::mm::{phys, virt};
use chos_lib::mm::{PAddr, PFrame, VAddr, VFrame, VFrameRange};

use super::phys::Page;

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

static ALL_MEMORY_REGIONS: [&'static (dyn MemoryRegion + Sync); 4] = [
    &IdentityMemoryRegion {
        name: "alloc",
        base: virt::PHYSICAL_MAP_BASE,
        typ: MemoryRegionType::Alloc,
    },
    &IdentityMemoryRegion {
        name: "heap",
        base: virt::HEAP_BASE,
        typ: MemoryRegionType::Normal,
    },
    &IdentityMemoryRegion {
        name: "iomem",
        base: virt::DEVICE_BASE,
        typ: MemoryRegionType::IoMem,
    },
    &StaticMemoryRegion {
        pbase: phys::KERNEL_DATA_BASE,
        vbase: virt::STATIC_BASE,
    },
];

fn get_memory_region_by_type(typ: MemoryRegionType) -> Option<&'static (dyn MemoryRegion + Sync)> {
    for &region in &ALL_MEMORY_REGIONS {
        if region.typ() == typ {
            return Some(region);
        }
    }
    None
}

fn get_memory_region_by_vaddr(vaddr: VAddr) -> Option<&'static (dyn MemoryRegion + Sync)> {
    for &region in &ALL_MEMORY_REGIONS {
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
