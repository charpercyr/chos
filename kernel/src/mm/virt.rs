use chos_config::arch::mm::{virt, phys};
use chos_lib::arch::mm::{PAddr, VAddr};

use super::phys::Page;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MemoryRegion {
    Alloc,
    Normal,
    PerCpu,
    IoMem,
    Static,
    Stack,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MemoryMapError {}

fn region_base(region: MemoryRegion) -> VAddr {
    match region {
        MemoryRegion::Alloc => virt::PHYSICAL_MAP_BASE,
        MemoryRegion::Normal => virt::HEAP_BASE,
        MemoryRegion::PerCpu => virt::PER_CPU_BASE,
        MemoryRegion::IoMem => virt::DEVICE_BASE,
        MemoryRegion::Static => virt::STATIC_BASE,
        MemoryRegion::Stack => virt::STACK_BASE,
    }
}

pub unsafe fn map_paddr(paddr: PAddr, region: MemoryRegion) -> Result<VAddr, MemoryMapError> {
    Ok(region_base(region) + paddr)
}

pub unsafe fn map_page(page: &Page, region: MemoryRegion) -> Result<VAddr, MemoryMapError> {
    map_paddr(page.paddr, region)
}

pub unsafe fn paddr_of(vaddr: VAddr, region: MemoryRegion) -> Option<PAddr> {
    match region {
        MemoryRegion::Static => Some(PAddr::new((vaddr - region_base(region)).as_u64()) + phys::KERNEL_DATA_BASE),
        MemoryRegion::PerCpu => todo!("Walk page table fpr {:#x}", vaddr),
        _ => Some(PAddr::new((vaddr - region_base(region)).as_u64()))
    }
}

pub unsafe fn unmap_page(_: VAddr) {
    // Nothing
}
