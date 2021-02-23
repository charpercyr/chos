
use core::hint::unreachable_unchecked;

#[repr(u32)]
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub enum QemuStatus {
    Success = 0x10,
    Error = 0x11,
}

pub fn exit_qemu(status: QemuStatus) -> ! {
    unsafe {
        let status = status as u32;
        asm! {
            "outl %eax, $0xf4",
            in("eax") status,
            options(att_syntax),
        };
        unreachable_unchecked()
    }
}