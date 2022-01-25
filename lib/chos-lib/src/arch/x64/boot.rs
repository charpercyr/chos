
#[derive(Clone, Copy, Debug)]
pub struct ArchKernelBootInfo {
    pub rsdt: usize,
    pub multiboot_header: usize,
}
