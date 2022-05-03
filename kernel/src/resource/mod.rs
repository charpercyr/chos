use alloc::string::String;
use core::mem::MaybeUninit;

use chos_lib::pool::{iarc_adapter, IArc, IArcCount};
use chos_lib::ReadOnly;

use crate::async_::oneshot::call_with_sender;
use crate::fs::buf::BufOwn;
use crate::fs::{self};
use crate::mm::slab::object_pool;
use crate::util::{private_impl, Private};

pub struct ResourceOps {
    pub file: fn(&ResourceArc) -> Option<FileArc>,
    pub dir: fn(&ResourceArc) -> Option<DirectoryArc>,
}

pub fn resource_no_file(_: &ResourceArc) -> Option<FileArc> {
    None
}

pub fn resource_no_dir(_: &ResourceArc) -> Option<DirectoryArc> {
    None
}

pub struct Resource {
    count: IArcCount,
    ops: &'static ResourceOps,
    private: Option<Private>,
}
iarc_adapter!(Resource: count);
object_pool!(pub struct ResourcePool : Resource);
pub type ResourceArc = IArc<Resource, ResourcePool>;

impl Resource {
    pub fn with_private(ops: &'static ResourceOps, user: Private) -> ResourceArc {
        Self {
            count: IArcCount::new(),
            ops,
            private: Some(user),
        }
        .into()
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
    count: IArcCount,
    ops: &'static FileOps,
    private: Option<Private>,
}
iarc_adapter!(File: count);
object_pool!(pub struct FilePool : File);
pub type FileArc = IArc<File, FilePool>;

impl File {
    pub fn with_private(ops: &'static FileOps, private: Private) -> FileArc {
        Self {
            count: IArcCount::new(),
            ops,
            private: Some(private),
        }
        .into()
    }
    private_impl!(private);
}

pub struct DirectoryEntry {
    pub name: String,
}

pub struct DirectoryOps {
    pub list: fn(
        &DirectoryArc,
        BufOwn<MaybeUninit<DirectoryEntry>>,
        fs::Sender<(usize, BufOwn<MaybeUninit<DirectoryEntry>>)>,
    ),
    pub mkfile: fn(
        &DirectoryArc,
        name: &str,
        
    ),
    pub mkdir: Option<()>,
}

pub struct Directory {
    count: IArcCount,
    ops: &'static DirectoryOps,
    private: Option<Private>,
}
iarc_adapter!(Directory: count);
object_pool!(pub struct DirectoryPool : Directory);
pub type DirectoryArc = IArc<Directory, DirectoryPool>;

impl Directory {
    pub fn with_private(ops: &'static DirectoryOps, private: Private) -> DirectoryArc {
        Self {
            count: IArcCount::new(),
            ops,
            private: Some(private),
        }
        .into()
    }

    pub fn list(
        self: &DirectoryArc,
        buf: BufOwn<MaybeUninit<DirectoryEntry>>,
        result: fs::Sender<(usize, BufOwn<MaybeUninit<DirectoryEntry>>)>,
    ) {
        (self.ops.list)(self, buf, result)
    }

    pub async fn async_list<'dir>(
        self: &DirectoryArc,
        buf: &'dir mut [MaybeUninit<DirectoryEntry>],
    ) -> fs::Result<&'dir mut [DirectoryEntry]> {
        let buf_own = unsafe { BufOwn::from_mut_slice(buf) };
        let (len, _) = call_with_sender!((Self::list)(self, buf_own)).await?;
        Ok(unsafe { MaybeUninit::slice_assume_init_mut(&mut buf[..len]) })
    }

    private_impl!(private);
}
