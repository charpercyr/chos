#[macro_export]
macro_rules! check_kernel_entry {
    ($entry:expr) => {
        const _: $crate::KernelEntry = $entry;
    };
}
