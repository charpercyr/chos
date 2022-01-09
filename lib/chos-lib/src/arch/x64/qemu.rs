use core::hint::unreachable_unchecked;

use crate::arch::port::PortWriteOnly;

#[repr(u32)]
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub enum QemuStatus {
    Success = 0x10,
    Error = 0x11,
}

impl QemuStatus {
    pub fn exit(self) -> ! {
        exit_qemu(self)
    }
}

pub fn exit_qemu(status: QemuStatus) -> ! {
    unsafe {
        PortWriteOnly::new(0xf4).write(status as u32);
        unreachable_unchecked()
    }
}
