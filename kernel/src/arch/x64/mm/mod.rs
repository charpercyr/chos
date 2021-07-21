
pub use chos_x64::paging::{
    PAGE_SIZE,
    PAGE_SIZE64,
    PAGE_MASK,
    PAGE_SHIFT,
    PAddr,
    VAddr,
};

enum PageFlushInner {
    None,
    PageRange {
        start: VAddr,
        end: VAddr,
    },
    Full,
}

#[must_use = "You must either call flush or discard"]
pub struct PageFlush {
    inner: PageFlushInner,
}

impl PageFlush {
    pub fn flush(self) {
        unsafe {
            match self.inner {
                PageFlushInner::None => (),
                PageFlushInner::PageRange { start, end } => {
                    let start = start.align_page().as_u64();
                    let end = end.align_page().as_u64();
                    let n = (end - start + 1) / PAGE_SIZE64;
                    for i in 0..n {
                        asm! {
                            "invpg ({addr})",
                            addr = in(reg) start + i * PAGE_SIZE64,
                        }
                    }
                },
                PageFlushInner::Full => asm! {
                    "mov %cr3, %rax",
                    "mov %rax, %cr3",
                    out("rax") _,
                    options(nomem, nostack, att_syntax),
                }
            }
        }
    }

    pub fn discard(self) {}
}
