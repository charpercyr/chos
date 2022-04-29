#[cfg(feature = "alloc")]
use alloc::borrow::Cow;
#[cfg(feature = "alloc")]
use alloc::format;
use core::mem::size_of;
use core::str::from_utf8;

use static_assertions::const_assert_eq;

use super::util::*;

const USTAR_SIG: &'static [u8; 6] = b"ustar\0";

pub enum FileLink {
    Normal,
    Link,
    Symlink,
}

#[repr(C)]
pub struct FileHeader {
    name: [u8; 100],
    mode: [u8; 8],
    uid: [u8; 8],
    gid: [u8; 8],
    size: [u8; 12],
    last_mod: [u8; 12],
    checksum: [u8; 8],
    typ: u8,
    link_name: [u8; 100],
    ustar_sig: [u8; 6],
    ustar_version: [u8; 2],
    user_name: [u8; 32],
    group_name: [u8; 32],
    dev_major: [u8; 8],
    dev_minor: [u8; 8],
    name_pre: [u8; 155],
}
const_assert_eq!(size_of::<FileHeader>(), 500);

impl FileHeader {
    pub fn name(&self) -> (&str, Option<&str>) {
        if &self.ustar_sig == USTAR_SIG {
            (
                from_utf8(trim_nulls(&self.name_pre)).expect("Invalid header"),
                Some(from_utf8(trim_nulls(&self.name_pre)).expect("Invalid header")),
            )
        } else {
            (
                from_utf8(trim_nulls(&self.name_pre)).expect("Invalid header"),
                None,
            )
        }
    }

    #[cfg(feature = "alloc")]
    pub fn name_merged(&self) -> Cow<'_, str> {
        match self.name() {
            (pre, Some(name)) => format!("{}{}", pre, name).into(),
            (name, None) => name.into(),
        }
    }

    pub fn size(&self) -> u64 {
        read_ascii_octal_trim(&self.size).expect("Invalid header")
    }
}
