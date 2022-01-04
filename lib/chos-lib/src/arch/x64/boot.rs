use super::acpi::Rsdt;

#[derive(Clone, Copy, Debug)]
pub struct ArchKernelBootInfo {
    pub rsdt: *const Rsdt,
    pub multiboot_header: usize,
}