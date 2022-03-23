use chos_config::arch::mm::virt;
use chos_lib::arch::acpi::Rsdt;

#[derive(Debug, Clone, Copy)]
pub struct ArchKernelArgs {
    pub rsdt: usize,
    pub mbh: usize,
}

impl ArchKernelArgs {
    pub unsafe fn rsdt(&self) -> Rsdt<'_> {
        Rsdt::new_offset(self.rsdt, virt::PHYSICAL_MAP_BASE.addr())
    }
}
