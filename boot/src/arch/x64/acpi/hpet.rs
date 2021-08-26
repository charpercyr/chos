use super::SDTHeader;

#[derive(Debug, Copy, Clone)]
#[repr(C, packed)]
pub struct HPET {
    pub hdr: SDTHeader,
    pub hw_rev_id: u8,
    pub flags: u8,
    pub pci_vendor_id: u16,
    pub address_space_id: u8,
    pub register_bit_width: u8,
    pub register_bit_offset: u8,
    res0: u8,
    pub address: usize,
    pub hpet_number: u8,
    pub minimum_tick: u16,
    pub page_protection: u8,
}

impl HPET {
    pub const SIGNATURE: &'static [u8; 4] = b"HPET";

    pub fn comparator_count(&self) -> u8 {
        (self.flags & 0b1111_1000) >> 3
    }
}
