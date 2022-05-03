use alloc::borrow::ToOwned;
use alloc::string::String;
use core::borrow::{Borrow, BorrowMut};
use core::intrinsics::transmute;
use core::ops::{Deref, DerefMut};

pub const SEPARATOR: char = '/';

#[repr(transparent)]
#[derive(Debug, PartialEq, Eq)]
pub struct Path {
    path: str,
}

impl Path {
    pub const fn new(path: &str) -> &Self {
        unsafe { transmute(path) }
    }

    pub const fn new_mut(path: &mut str) -> &mut Self {
        unsafe { transmute(path) }
    }

    pub const fn as_str(&self) -> &str {
        &self.path
    }

    pub const fn as_str_mut(&mut self) -> &mut str {
        &mut self.path
    }

    pub fn is_absolute(&self) -> bool {
        self.path.starts_with(SEPARATOR)
    }

    pub fn is_relative(&self) -> bool {
        !self.is_absolute()
    }
}

impl ToOwned for Path {
    type Owned = PathBuf;
    fn to_owned(&self) -> Self::Owned {
        self.into()
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct PathBuf {
    path: String,
}

impl PathBuf {
    pub const fn new() -> Self {
        Self {
            path: String::new(),
        }
    }
}

impl From<&Path> for PathBuf {
    fn from(p: &Path) -> Self {
        Self {
            path: p.path.into(),
        }
    }
}

impl Deref for PathBuf {
    type Target = Path;
    fn deref(&self) -> &Self::Target {
        Path::new(&self.path)
    }
}

impl DerefMut for PathBuf {
    fn deref_mut(&mut self) -> &mut Self::Target {
        Path::new_mut(&mut self.path)
    }
}

impl Borrow<Path> for PathBuf {
    fn borrow(&self) -> &Path {
        &*self
    }
}

impl BorrowMut<Path> for PathBuf {
    fn borrow_mut(&mut self) -> &mut Path {
        &mut *self
    }
}

impl AsRef<Path> for PathBuf {
    fn as_ref(&self) -> &Path {
        &*self
    }
}
impl AsMut<Path> for PathBuf {
    fn as_mut(&mut self) -> &mut Path {
        &mut *self
    }
}
