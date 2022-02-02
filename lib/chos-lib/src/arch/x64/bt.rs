use core::arch::asm;

use super::mm::VAddr;

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
    type Item = VAddr;

    fn next(&mut self) -> Option<Self::Item> {
        unsafe {
            if !self.frame.is_null() {
                let ptr = (*self.frame).ip;
                self.frame = (*self.frame).next;
                Some(VAddr::new_unchecked(ptr as u64))
            } else {
                None
            }
        }
    }
}

#[inline(always)]
pub fn backtrace() -> Backtrace {
    let mut frame: *const Frame;
    unsafe {
        asm!(
            "mov {}, rbp",
            out(reg) frame,
        );
    }
    Backtrace { frame }
}
