use chos_lib::arch::acpi::Rsdt;

#[derive(Debug, Clone, Copy)]
pub struct ArchKernelArgs {
    pub rsdt: *const Rsdt,
}