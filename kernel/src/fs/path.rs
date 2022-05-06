use alloc::borrow::ToOwned;
use alloc::string::String;
use core::borrow::{Borrow, BorrowMut};
use core::fmt;
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

    pub fn components(&self) -> Components<'_> {
        Components {
            path: self.as_str(),
        }
    }

    pub fn is_absolute(&self) -> bool {
        self.path.starts_with(SEPARATOR)
    }

    pub fn is_relative(&self) -> bool {
        !self.is_absolute()
    }
}

impl fmt::Display for Path {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(&self.path, f)
    }
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum Component<'a> {
    RootDir,
    CurDir,
    ParentDir,
    Normal(&'a str),
}

#[derive(Copy, Clone)]
pub struct Components<'a> {
    path: &'a str,
}

impl<'a> Iterator for Components<'a> {
    type Item = Component<'a>;
    fn next(&mut self) -> Option<Self::Item> {
        if self.path.is_empty() {
            return None;
        }
        let mut sep_idx = self.path.find(SEPARATOR).unwrap_or(self.path.len());
        let component = match &self.path[..sep_idx] {
            "" => Component::RootDir,
            "." => Component::CurDir,
            ".." => Component::ParentDir,
            normal => Component::Normal(normal),
        };
        while (&self.path[sep_idx..]).starts_with(SEPARATOR) {
            sep_idx += SEPARATOR.len_utf8();
        }
        self.path = &self.path[sep_idx..];
        Some(component)
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
