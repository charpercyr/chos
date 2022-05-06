pub mod buf;
pub mod path;

use alloc::sync::Arc;

use bitflags::bitflags;
use chos_lib::intrusive::hash_table::{sizes, AtomicLink, HashTable};
use chos_lib::log::debug;
use chos_lib::pool::{iarc_adapter_weak, IArc, IArcCountWeak, IWeak};
use chos_lib::sync::SpinRWLock;
use intrusive_collections::{intrusive_adapter, KeyAdapter};

use crate::async_::oneshot::{self, call_with_sender};
use crate::driver::block::BlockDevice;
use crate::mm::slab::object_pool;
use crate::module::Module;
use crate::resource::ResourceArc;
use crate::util::{private_impl, Private};

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

    pub const fn new_with_private(
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
    count: IArcCountWeak,
    ops: &'static SuperblockOps,
    private: Option<Private>,
}
iarc_adapter_weak!(Superblock: count);
object_pool!(pub struct SuperblockPool : Superblock);
pub type SuperblockArc = IArc<Superblock, SuperblockPool>;
pub type SuperblockWeak = IWeak<Superblock, SuperblockPool>;
unsafe impl Send for Superblock {}
unsafe impl Sync for Superblock {}

impl Superblock {
    pub const fn new(ops: &'static SuperblockOps) -> Self {
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

bitflags! {
    pub struct InodeMode : u32 {
        const OTH_EX =  0b000_000_000_001;
        const OTH_WR =  0b000_000_000_010;
        const OTH_RD =  0b000_000_000_100;

        const GRP_EX =  0b000_000_001_000;
        const GRP_WR =  0b000_000_010_000;
        const GRP_RD =  0b000_000_100_000;

        const OWN_EX =  0b000_001_000_000;
        const OWN_WR =  0b000_010_000_000;
        const OWN_RD =  0b000_100_000_000;

        const STICKY =  0b001_000_000_000;
        const SET_GID = 0b010_000_000_000;
        const SET_UID = 0b100_000_000_000;
    }
}

impl InodeMode {
    pub const OWN_RW: Self = Self {
        bits: Self::OWN_RD.bits() | Self::OWN_WR.bits(),
    };
    pub const OWN_RX: Self = Self {
        bits: Self::OWN_RD.bits() | Self::OWN_EX.bits(),
    };
    pub const OWN_RWX: Self = Self {
        bits: Self::OWN_RD.bits() | Self::OWN_WR.bits() | Self::OWN_EX.bits(),
    };

    pub const GRP_RW: Self = Self {
        bits: Self::GRP_RD.bits() | Self::GRP_WR.bits(),
    };
    pub const GRP_RX: Self = Self {
        bits: Self::GRP_RD.bits() | Self::GRP_EX.bits(),
    };
    const GRP_RWX: Self = Self {
        bits: Self::GRP_RD.bits() | Self::GRP_WR.bits() | Self::GRP_EX.bits(),
    };

    pub const OTH_RW: Self = Self {
        bits: Self::OTH_RD.bits() | Self::OTH_WR.bits(),
    };
    pub const OTH_RX: Self = Self {
        bits: Self::OTH_RD.bits() | Self::OTH_EX.bits(),
    };
    pub const OTH_RWX: Self = Self {
        bits: Self::OTH_RD.bits() | Self::OTH_WR.bits() | Self::OTH_EX.bits(),
    };

    pub const DEFAULT_FILE: Self = Self {
        bits: Self::OWN_RW.bits() | Self::GRP_RD.bits() | Self::OTH_RD.bits(),
    };
    pub const DEFAULT_DIR: Self = Self {
        bits: Self::OWN_RWX.bits() | Self::GRP_RX.bits() | Self::OTH_RX.bits(),
    };
    pub const DEFAULT_EXE: Self = Self {
        bits: Self::OWN_RWX.bits() | Self::GRP_RX.bits() | Self::OTH_RX.bits(),
    };
}

pub struct InodeAttributes {
    pub mode: InodeMode,
    pub uid: u32,
    pub gid: u32,
}

impl InodeAttributes {
    pub const fn empty() -> Self {
        Self::root(InodeMode::empty())
    }

    pub const fn root(mode: InodeMode) -> Self {
        Self {
            mode,
            uid: 0,
            gid: 0,
        }
    }
}

pub struct Inode {
    count: IArcCountWeak,
    ops: &'static InodeOps,
    pub attrs: InodeAttributes,
    private: Option<Private>,
}
iarc_adapter_weak!(Inode: count);
object_pool!(pub struct InodePool : Inode);
pub type InodeArc = IArc<Inode, InodePool>;
pub type InodeWeak = IWeak<Inode, InodePool>;

impl Inode {
    pub const fn new(ops: &'static InodeOps) -> Self {
        Self {
            count: IArcCountWeak::new(),
            ops,
            attrs: InodeAttributes::empty(),
            private: None,
        }
    }
    
    pub fn with_attributes(self, attrs: InodeAttributes) -> Self {
        Self {
            attrs,
            ..self
        }
    }

    pub fn with_private(self, private: Private) -> Self {
        Self {
            private: Some(private),
            ..self
        }
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
