mod mapper;
mod paging;

pub use mapper::*;
pub use paging::*;

pub const PAGE_SHIFT: u32 = 12;
pub const PAGE_MASK: u64 = (1 << PAGE_SHIFT) - 1;
pub const PAGE_SIZE: usize = 1 << PAGE_SHIFT;
pub const PAGE_SIZE64: u64 = 1 << PAGE_SHIFT;

pub type PAddrRepr = u64;
pub type VAddrRepr = u64;

const CANONICAL_SHIFT: u8 = 47;
const CANONICAL_MASK: VAddrRepr = !((1 << CANONICAL_SHIFT) - 1);

pub const fn is_canonical(addr: VAddrRepr) -> bool {
    (addr & CANONICAL_MASK) == 0 || (addr & CANONICAL_MASK) == CANONICAL_MASK
}

pub const fn make_canonical(addr: VAddrRepr) -> VAddrRepr {
    if addr & CANONICAL_MASK == 0 {
        addr
    } else {
        addr | CANONICAL_MASK
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_is_canonical() {
        assert!(is_canonical(0x0000_0000_0000_0000));
        assert!(is_canonical(0xffff_8000_0000_0000));
        assert!(is_canonical(0xffff_ffff_ffff_ffff));
        assert!(!is_canonical(0x8000_0000_0000_0000));
        assert!(!is_canonical(0xff00_0000_0000_0000));
    }

    #[test]
    fn test_make_canonical() {
        assert_eq!(make_canonical(0x0000_0000_0000_0000), 0x0000_0000_0000_0000);
        assert_eq!(make_canonical(0xffff_8000_0000_0000), 0xffff_8000_0000_0000);
        assert_eq!(make_canonical(0xffff_ffff_ffff_ffff), 0xffff_ffff_ffff_ffff);
        assert_eq!(make_canonical(0x0000_8000_0000_0000), 0xffff_8000_0000_0000);
        assert_eq!(make_canonical(0x0000_7fff_ffff_ffff), 0x0000_7fff_ffff_ffff);
        assert_eq!(make_canonical(0x0000_ffff_ffff_ffff), 0xffff_ffff_ffff_ffff);
    }
}