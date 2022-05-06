#[cfg(feature = "alloc")]
use alloc::borrow::Cow;
#[cfg(feature = "alloc")]
use alloc::format;
use core::mem::size_of;
use core::str::from_utf8;

use static_assertions::const_assert_eq;

use super::util::*;

const USTAR_SIG: &'static [u8; 6] = b"ustar ";

pub enum FileLink {
    Normal,
    Link,
    Symlink,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EntryType {
    File,
    Link,
    Symlink,
    Dir,
    Unknown(u8),
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
        if self.is_ustar() {
            let name_pre = from_utf8(trim_nulls(&self.name_pre)).expect("Invalid header");
            if name_pre.is_empty() {
                (
                    from_utf8(trim_nulls(&self.name)).expect("Invalid header"),
                    None,
                )
            } else {
                (
                    name_pre,
                    Some(from_utf8(trim_nulls(&self.name_pre)).expect("Invalid header")),
                )
            }
        } else {
            (
                from_utf8(trim_nulls(&self.name)).expect("Invalid header"),
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

    pub fn typ(&self) -> EntryType {
        match self.typ {
            0 | b'0' => EntryType::File,
            b'1' => EntryType::Link,
            b'2' => EntryType::Symlink,
            b'5' => EntryType::Dir,
            t => EntryType::Unknown(t),
        }
    }

    pub fn uid(&self) -> u32 {
        read_ascii_octal_trim(&self.uid).expect("Invalid header") as u32
    }

    pub fn user_name(&self) -> Option<&str> {
        self.is_ustar()
            .then(|| from_utf8(trim_nulls(&self.user_name)).expect("Invalid header"))
    }

    pub fn gid(&self) -> u32 {
        read_ascii_octal_trim(&self.gid).expect("Invalid header") as u32
    }

    pub fn group_name(&self) -> Option<&str> {
        self.is_ustar()
            .then(|| from_utf8(trim_nulls(&self.group_name)).expect("Invalid header"))
    }

    pub fn mode(&self) -> u32 {
        read_ascii_octal_trim(&self.mode).expect("Invalid header") as u32
    }

    fn is_ustar(&self) -> bool {
        &self.ustar_sig == USTAR_SIG
    }
}
