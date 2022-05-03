pub mod buf;
pub mod path;

use alloc::sync::Arc;

use chos_lib::intrusive::hash_table::{sizes, AtomicLink, HashTable};
use chos_lib::log::debug;
use chos_lib::pool::{iarc_adapter, IArc, IArcCount};
use chos_lib::sync::SpinRWLock;
use intrusive_collections::{intrusive_adapter, KeyAdapter};

use crate::async_::oneshot::{self, call_with_sender};
use crate::driver::block::BlockDevice;
use crate::mm::slab::object_pool;
use crate::module::Module;
use crate::resource::ResourceArc;
use crate::util::{Private, private_impl};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[repr(usize)]
pub enum Error {
    AllocError,
    InvalidArgument,
    NotSupported,
}
pub type Result<T> = core::result::Result<T, Error>;
pub type Receiver<T> = oneshot::Receiver<Result<T>>;
pub type Sender<T> = oneshot::Sender<Result<T>>;

pub struct FilesystemOps {
    pub mount: fn(&Filesystem, Option<Arc<dyn BlockDevice>>, Sender<SuperblockArc>),
}

pub struct Filesystem {
    link: AtomicLink,
    pub name: &'static str,
    ops: &'static FilesystemOps,
    private: Option<Private>,
}
intrusive_adapter!(FilesystemAdapter = &'static Filesystem: Filesystem { link: AtomicLink });

unsafe impl Send for Filesystem {}
unsafe impl Sync for Filesystem {}

impl<'a> KeyAdapter<'a> for FilesystemAdapter {
    type Key = &'a str;
    fn get_key(&self, value: &'a Filesystem) -> &'a str {
        value.name
    }
}

impl Filesystem {
    pub const fn new(name: &'static str, ops: &'static FilesystemOps) -> Self {
        Self {
            link: AtomicLink::new(),
            name,
            ops,
            private: None,
        }
    }

    pub const fn with_private(
        name: &'static str,
        ops: &'static FilesystemOps,
        private: Private,
    ) -> Self {
        Self {
            link: AtomicLink::new(),
            name,
            ops,
            private: Some(private),
        }
    }

    pub fn user<T: 'static>(&self) -> Option<&T> {
        self.private.as_ref()?.downcast_ref()
    }

    pub fn mount(&self, blkdev: Option<Arc<dyn BlockDevice>>, result: Sender<SuperblockArc>) {
        (self.ops.mount)(self, blkdev, result)
    }
    
    private_impl!(private);
}

pub struct SuperblockOps {
    pub root: fn(&SuperblockArc, Sender<InodeArc>),
}

pub struct Superblock {
    count: IArcCount,
    ops: &'static SuperblockOps,
    private: Option<Private>,
}
iarc_adapter!(Superblock: count);
object_pool!(pub struct SuperblockPool : Superblock);
pub type SuperblockArc = IArc<Superblock, SuperblockPool>;
unsafe impl Send for Superblock {}
unsafe impl Sync for Superblock {}

impl Superblock {
    pub fn with_private(
        ops: &'static SuperblockOps,
        private: Private,
    ) -> SuperblockArc {
        Self {
            count: IArcCount::new(),
            ops,
            private: Some(private),
        }
        .into()
    }

    pub fn root(self: &SuperblockArc, result: Sender<InodeArc>) {
        (self.ops.root)(self, result)
    }

    pub async fn async_root(self: &SuperblockArc) -> Result<InodeArc> {
        call_with_sender!((Self::root)(self)).await
    }

    private_impl!(private);
}

pub struct InodeOps {
    pub open: fn(&InodeArc, Sender<ResourceArc>),
}

pub struct Inode {
    count: IArcCount,
    ops: &'static InodeOps,
    private: Option<Private>,
}
iarc_adapter!(Inode: count);
object_pool!(pub struct InodePool : Inode);
pub type InodeArc = IArc<Inode, InodePool>;

impl Inode {
    pub fn with_private(ops: &'static InodeOps, private: Private) -> InodeArc {
        Self {
            count: IArcCount::new(),
            ops,
            private: Some(private),
        }
        .into()
    }

    pub fn open(self: &InodeArc, result: Sender<ResourceArc>) {
        (self.ops.open)(self, result)
    }

    pub async fn async_open(self: &InodeArc) -> Result<ResourceArc> {
        call_with_sender!((Self::open)(self)).await
    }

    private_impl!(private);
}

static FILESYSTEMS: SpinRWLock<HashTable<FilesystemAdapter, sizes::O4>> =
    SpinRWLock::new(HashTable::new(FilesystemAdapter::NEW));

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct NoSuchFilesystem;

pub fn with_filesystem<R>(
    name: &str,
    f: impl FnOnce(&Filesystem) -> R,
) -> core::result::Result<R, NoSuchFilesystem> {
    let fss = FILESYSTEMS.lock_read();
    fss.find(&name).get().map(f).ok_or(NoSuchFilesystem)
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct FilesystemAlreadyExists;

pub fn register_filesystem(
    fs: &'static Filesystem,
    _module: Module,
) -> core::result::Result<(), FilesystemAlreadyExists> {
    debug!("Register filesystem '{}'", fs.name);
    let mut fss = FILESYSTEMS.lock_write();
    if fss.find(&fs.name).is_valid() {
        debug!("Filesystem '{}' already exists", fs.name);
        return Err(FilesystemAlreadyExists);
    }
    fss.insert(fs);
    Ok(())
}

pub fn unregister_filesystem(
    fs: &'static Filesystem,
) -> core::result::Result<(), NoSuchFilesystem> {
    debug!("Unregister filesystem '{}'", fs.name);
    let mut fss = FILESYSTEMS.lock_write();
    fss.find_mut(&fs.name)
        .unlink()
        .map(|_| ())
        .ok_or(NoSuchFilesystem)
}
