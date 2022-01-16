use chos_lib::arch::acpi::Rsdt;

#[derive(Debug)]
pub struct ArchKernelArgs {
    pub rsdt: *const Rsdt,
}