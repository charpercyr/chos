use alloc::string::String;
use core::mem::MaybeUninit;

use chos_lib::pool::{iarc_adapter_weak, IArc, IArcCountWeak, IWeak};
use chos_lib::ReadOnly;

use crate::async_::oneshot::call_with_sender;
use crate::fs::buf::BufOwn;
use crate::fs::{self, InodeArc, InodeAttributes, InodeWeak};
use crate::mm::slab::object_pool;
use crate::util::{private_impl, Private};

pub struct ResourceOps {
    pub file: fn(&ResourceArc) -> Option<FileArc>,
    pub dir: fn(&ResourceArc) -> Option<DirectoryArc>,
}

pub struct Resource {
    count: IArcCountWeak,
    ops: &'static ResourceOps,
    pub inode: Option<InodeWeak>,
    pub parent: Option<InodeWeak>,
    private: Option<Private>,
}
iarc_adapter_weak!(Resource: count);
object_pool!(pub struct ResourcePool : Resource);
pub type ResourceArc = IArc<Resource, ResourcePool>;
pub type ResourceWeak = IWeak<Resource, ResourcePool>;

impl Resource {
    pub const fn new(ops: &'static ResourceOps) -> Self {
        Self {
            count: IArcCountWeak::new(),
            ops,
            inode: None,
            parent: None,
            private: None,
        }
    }

    pub fn with_private(self, private: Private) -> Self {
        Self {
            private: Some(private),
            ..self
        }
    }

    pub fn with_inode(self, inode: InodeWeak) -> Self {
        Self {
            inode: Some(inode),
            ..self
        }
    }

    pub fn with_parent(self, parent: InodeWeak) -> Self {
        Self {
            parent: Some(parent),
            ..self
        }
    }

    pub fn file(self: &ResourceArc) -> Option<FileArc> {
        (self.ops.file)(self)
    }

    pub fn dir<'a>(self: &ResourceArc) -> Option<DirectoryArc> {
        (self.ops.dir)(self)
    }

    private_impl!(private);
}

pub struct FileOps {
    pub read: fn(&FileArc, BufOwn<u8>, fs::Sender<(usize, BufOwn<u8>)>),
    pub write: fn(&FileArc, BufOwn<u8, ReadOnly>, fs::Sender<(usize, BufOwn<u8, ReadOnly>)>),
}

pub struct File {
    count: IArcCountWeak,
    ops: &'static FileOps,
    private: Option<Private>,
}
iarc_adapter_weak!(File: count);
object_pool!(pub struct FilePool : File);
pub type FileArc = IArc<File, FilePool>;
pub type FileWeak = IWeak<File, FilePool>;

impl File {
    pub const fn new(ops: &'static FileOps) -> Self {
        Self {
            count: IArcCountWeak::new(),
            ops,
            private: None,
        }
    }

    pub fn with_private(self, private: Private) -> Self {
        Self {
            private: Some(private),
            ..self
        }
    }

    private_impl!(private);
}

pub struct DirectoryEntry {
    pub name: String,
    pub attrs: InodeAttributes,
    pub inode: InodeArc,
}

pub struct DirectoryOps {
    pub list: fn(
        &DirectoryArc,
        BufOwn<MaybeUninit<DirectoryEntry>>,
        fs::Sender<(usize, BufOwn<MaybeUninit<DirectoryEntry>>)>,
    ),
    pub mkfile: Option<fn(&DirectoryArc, &str, InodeAttributes, fs::Sender<FileArc>)>,
    pub mkdir: Option<fn(&DirectoryArc, &str, InodeAttributes, fs::Sender<DirectoryArc>)>,
}

pub struct Directory {
    count: IArcCountWeak,
    ops: &'static DirectoryOps,
    private: Option<Private>,
}
iarc_adapter_weak!(Directory: count);
object_pool!(pub struct DirectoryPool : Directory);
pub type DirectoryArc = IArc<Directory, DirectoryPool>;
pub type DirectoryWeak = IWeak<Directory, DirectoryPool>;

impl Directory {
    pub const fn new(ops: &'static DirectoryOps) -> Self {
        Self {
            count: IArcCountWeak::new(),
            ops,
            private: None,
        }
    }

    pub fn with_private(self, private: Private) -> Self {
        Self {
            private: Some(private),
            ..self
        }
    }

    pub fn list(
        self: &DirectoryArc,
        buf: BufOwn<MaybeUninit<DirectoryEntry>>,
        result: fs::Sender<(usize, BufOwn<MaybeUninit<DirectoryEntry>>)>,
    ) {
        (self.ops.list)(self, buf, result)
    }

    pub async fn async_list(
        self: &DirectoryArc,
        buf: BufOwn<MaybeUninit<DirectoryEntry>>,
    ) -> fs::Result<(usize, BufOwn<MaybeUninit<DirectoryEntry>>)> {
        call_with_sender!((Self::list)(self, buf)).await
    }

    pub fn mkfile(
        self: &DirectoryArc,
        name: &str,
        attrs: InodeAttributes,
        result: fs::Sender<FileArc>,
    ) {
        if let Some(mkfile) = self.ops.mkfile {
            mkfile(self, name, attrs, result)
        } else {
            result.send(Err(fs::Error::NotSupported));
        }
    }

    pub async fn async_mkfile(
        self: &DirectoryArc,
        name: &str,
        attrs: InodeAttributes,
    ) -> fs::Result<FileArc> {
        call_with_sender!((Self::mkfile)(self, name, attrs)).await
    }

    pub fn mkdir(
        self: &DirectoryArc,
        name: &str,
        attrs: InodeAttributes,
        result: fs::Sender<DirectoryArc>,
    ) {
        if let Some(mkdir) = self.ops.mkdir {
            mkdir(self, name, attrs, result)
        } else {
            result.send(Err(fs::Error::NotSupported));
        }
    }

    pub async fn async_mkdir(
        self: &DirectoryArc,
        name: &str,
        attrs: InodeAttributes,
    ) -> fs::Result<DirectoryArc> {
        call_with_sender!((Self::mkdir)(self, name, attrs)).await
    }

    private_impl!(private);
}
