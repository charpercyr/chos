use core::ptr::null;

#[repr(C, packed)]
struct Frame {
    next: *const Frame,
    ip: *const (),
}

#[derive(Copy, Clone, Debug)]
pub struct Backtrace {
    frame: *const Frame,
}

impl Iterator for Backtrace {
    type Item = *const ();

    fn next(&mut self) -> Option<Self::Item> {
        unsafe {
            if self.frame != null() {
                let ptr = (*self.frame).ip;
                self.frame = (*self.frame).next;
                Some(ptr)
            } else {
                None
            }
        }
    }
}

#[inline(always)]
pub unsafe fn backtrace() -> Backtrace {
    let mut frame: *const Frame;
    asm!(
        "mov {}, rbp",
        out(reg) frame,
    );
    Backtrace { frame }
}
