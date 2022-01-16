use core::mem::size_of;

use static_assertions::const_assert_eq;


#[repr(C)]
pub struct FileHeader {
    pub name: [u8; 100],
    pub mode: [u8; 8],
    pub uid: [u8; 8],
    pub gid: [u8; 8],
    pub size: [u8; 12],
    pub last_mod: [u8; 12],
    pub checksum: [u8; 8],
    pub typ: u8,
    pub link_name: [u8; 100],
}

#[repr(C)]
pub struct UStarFileHeader {
    pub hdr: FileHeader,
    pub ustar_sig: [u8; 6],
    pub ustar_version: [u8; 2],
    pub user_name: [u8; 32],
    pub group_name: [u8; 32],
    pub dev_major: [u8; 8],
    pub dev_minor: [u8; 8],
    pub name_pre: [u8; 155],
}
const_assert_eq!(size_of::<UStarFileHeader>(), 500);

