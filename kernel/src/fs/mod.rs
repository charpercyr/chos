pub mod buf;

use alloc::boxed::Box;
use alloc::sync::Arc;
use core::ptr::NonNull;

use chos_lib::intrusive::hash_table::{sizes, AtomicLink, HashTable};
use chos_lib::log::debug;
use chos_lib::sync::SpinRWLock;
use intrusive_collections::{intrusive_adapter, KeyAdapter};

use crate::async_::oneshot::{self, call_with_sender};
use crate::driver::block::BlockDevice;
use crate::module::Module;
use crate::resource::Resource;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Error {
    AllocError,
    InvalidArgument,
}
pub type Result<T> = core::result::Result<T, Error>;
pub type Receiver<T> = oneshot::Receiver<Result<T>>;
pub type Sender<T> = oneshot::Sender<Result<T>>;

pub struct FilesystemOps {
    pub mount: fn(&Filesystem, Option<Arc<dyn BlockDevice>>, Sender<Box<dyn Superblock>>),
}

pub struct Filesystem {
    link: AtomicLink,
    pub name: &'static str,
    pub ops: &'static FilesystemOps,
    pub user: Option<NonNull<()>>,
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
            user: None,
        }
    }

    pub const fn with_user(self, user: NonNull<()>) -> Self {
        Self {
            user: Some(user),
            ..self
        }
    }

    pub async fn mount(&self, blkdev: Option<Arc<dyn BlockDevice>>) -> Result<Box<dyn Superblock>> {
        let (sender, recv) = oneshot::channel();
        (self.ops.mount)(self, blkdev, sender);
        recv.await
    }
}

pub trait Superblock: Send + Sync {
    fn root(&self, result: Sender<Arc<dyn Inode>>);
}

impl dyn Superblock {
    pub async fn async_root(&self) -> Result<Arc<dyn Inode>> {
        call_with_sender!((Superblock::root)(self)).await
    }
}

pub trait Inode: Send + Sync {
    fn open(&self, result: Sender<Arc<dyn Resource>>);
}

impl dyn Inode {
    pub async fn async_open(&self) -> Result<Arc<dyn Resource>> {
        call_with_sender!((Inode::open)(self)).await
    }
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
