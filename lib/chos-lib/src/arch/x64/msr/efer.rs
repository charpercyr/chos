use bitflags::bitflags;

use super::Msr;

const EFER: Msr = Msr::new(0xc0000080);

bitflags! {
    pub struct EferFlags: u64 {
        const SYSTEM_CALL_EXTENSIONS = 1 << 0;
        const LONG_MODE_ENABLE = 1 << 8;
        const LONG_MODE_ACTIVE = 1 << 10;
        const NO_EXECUTE_ENABLE = 1 << 11;
        const SECURE_VIRTUAL_MACHINE_ENABLE = 1 << 12;
        const LONG_MODE_SEGMENT_LIMIT_ENABLE = 1 << 13;
        const FAST_FXSAVE_FXRSTOR = 1 << 14;
        const TRANSLATION_CACHE_EXTENSION = 1 << 15;
    }
}

pub struct Efer;
impl Efer {
    pub fn read() -> EferFlags {
        EferFlags::from_bits_truncate(unsafe { EFER.read_raw() })
    }

    pub unsafe fn write(efer: EferFlags) {
        EFER.write_raw_shared(efer.bits())
    }
}
