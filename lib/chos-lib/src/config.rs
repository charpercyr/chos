pub mod domain {
    use crate::log::domain;

    #[cfg(target_arch = "x86_64")]
    domain! {
        GDT = false,
        IDT = false,
        TSS = false,
        PAGE_TABLE = false,
    }
}
