#![no_std]
#![feature(try_blocks)]

extern crate alloc;

use alloc::sync::Arc;
use alloc::vec::Vec;
use chos_lib::log::todo_warn;
use core::mem::MaybeUninit;

use chos::driver::block::BlockDevice;
use chos::fs::buf::BufOwn;
use chos::fs::{
    self, register_filesystem, unregister_filesystem, Filesystem, FilesystemOps, Inode, InodeArc,
    InodeAttributes, InodeMode, InodeOps, InodeWeak, Superblock, SuperblockArc, SuperblockOps,
};
use chos::module::{module_decl, Module, ModuleDecl};
use chos::resource::{
    Directory, DirectoryArc, DirectoryEntry, DirectoryOps, File, FileArc, FileOps, Resource,
    ResourceArc, ResourceOps, ResourceWeak,
};
use chos_lib::sync::Spinlock;
use chos_lib::ReadOnly;

struct RamfsFile {}

impl RamfsFile {
    fn new(resource: ResourceWeak) -> File {
        File::new(&RAMFS_FILE_OPS, resource).with_private(Arc::new(RamfsFile {}))
    }
}

fn ramfs_file_read(
    _file: &FileArc,
    _offset: usize,
    _buf: BufOwn<u8>,
    result: fs::Sender<(usize, BufOwn<u8>)>,
) {
    todo_warn!("ramfs_file_read");
    result.send_err(fs::Error::NotSupported)
}

fn ramfs_file_write(
    _file: &FileArc,
    _offset: usize,
    _buf: BufOwn<u8, ReadOnly>,
    result: fs::Sender<(usize, BufOwn<u8, ReadOnly>)>,
) {
    todo_warn!("ramfs_file_write");
    result.send_err(fs::Error::NotSupported)
}

static RAMFS_FILE_OPS: FileOps = FileOps {
    read: ramfs_file_read,
    write: ramfs_file_write,
};

struct RamfsDir {
    children: Spinlock<Vec<DirectoryEntry>>,
}

impl RamfsDir {
    fn new(resource: ResourceWeak) -> Directory {
        Directory::new(&RAMFS_DIR_OPS, resource).with_private(Arc::new(RamfsDir {
            children: Spinlock::new(Vec::new()),
        }))
    }
}

fn ramfs_dir_list(
    dir: &DirectoryArc,
    idx: usize,
    mut buf: BufOwn<MaybeUninit<DirectoryEntry>>,
    result: fs::Sender<(usize, BufOwn<MaybeUninit<DirectoryEntry>>)>,
) {
    let private = dir.private::<RamfsDir>().unwrap();
    let resource = dir.resource.upgrade().unwrap();
    let inode = resource.inode.as_ref().unwrap().upgrade().unwrap();
    let mut total = 0;
    if idx <= 0 && buf.len() >= 1 {
        buf[0] = MaybeUninit::new(DirectoryEntry {
            name: ".".into(),
            inode: inode.clone(),
        });
        total += 1;
    }
    if idx <= 1 && buf.len() >= 2 {
        let parent = inode
            .parent
            .as_ref()
            .map(|parent| parent.upgrade().unwrap())
            .unwrap_or_else(|| inode.clone());
        buf[1] = MaybeUninit::new(DirectoryEntry {
            name: "..".into(),
            inode: parent,
        });
        total += 1;
    }
    let children = private.children.lock();
    for i in idx.max(2)..children.len() {
        buf[i] = MaybeUninit::new(children[i - 2].clone());
        total += 1;
    }
    result.send_ok((total, buf))
}

fn ramfs_dir_mkfile(
    dir: &DirectoryArc,
    name: &str,
    attrs: InodeAttributes,
    result: fs::Sender<FileArc>,
) {
    let private = dir.private::<RamfsDir>().unwrap();
    let mut children = private.children.lock();
    let inode =
        InodeArc::new_cyclic(|inode| RamfsInode::new(RamfsResource::file(inode.clone()), attrs));
    let file = inode.private::<RamfsInode>().unwrap().res.file().unwrap();
    children.push(DirectoryEntry {
        name: name.into(),
        inode,
    });
    result.send_ok(file);
}

fn ramfs_dir_mkdir(
    dir: &DirectoryArc,
    name: &str,
    attrs: InodeAttributes,
    result: fs::Sender<DirectoryArc>,
) {
    let private = dir.private::<RamfsDir>().unwrap();
    let mut children = private.children.lock();
    let inode =
        InodeArc::new_cyclic(|inode| RamfsInode::new(RamfsResource::dir(inode.clone()), attrs));
    let file = inode.private::<RamfsInode>().unwrap().res.dir().unwrap();
    children.push(DirectoryEntry {
        name: name.into(),
        inode,
    });
    result.send_ok(file);
}

static RAMFS_DIR_OPS: DirectoryOps = DirectoryOps {
    list_iter: ramfs_dir_list,
    mkfile: Some(ramfs_dir_mkfile),
    mkdir: Some(ramfs_dir_mkdir),
};

enum RamfsResource {
    File(FileArc),
    Dir(DirectoryArc),
}

impl RamfsResource {
    pub fn file(inode: InodeWeak) -> ResourceArc {
        ResourceArc::new_cyclic(|res| {
            Resource::new(&RAMFS_RES_OPS)
                .with_inode(inode)
                .with_private(Arc::new(RamfsResource::File(
                    RamfsFile::new(res.clone()).into(),
                )))
        })
    }

    pub fn dir(inode: InodeWeak) -> ResourceArc {
        ResourceArc::new_cyclic(|res| {
            Resource::new(&RAMFS_RES_OPS)
                .with_inode(inode)
                .with_private(Arc::new(RamfsResource::Dir(
                    RamfsDir::new(res.clone()).into(),
                )))
        })
    }
}

fn ramfs_res_file(res: &ResourceArc) -> Option<FileArc> {
    let private = res.private::<RamfsResource>().unwrap();
    if let RamfsResource::File(file) = private {
        Some(file.clone())
    } else {
        None
    }
}

fn ramfs_res_dir(res: &ResourceArc) -> Option<DirectoryArc> {
    let private = res.private::<RamfsResource>().unwrap();
    if let RamfsResource::Dir(dir) = private {
        Some(dir.clone())
    } else {
        None
    }
}

static RAMFS_RES_OPS: ResourceOps = ResourceOps {
    dir: ramfs_res_dir,
    file: ramfs_res_file,
};

struct RamfsInode {
    res: ResourceArc,
}

impl RamfsInode {
    pub fn new(res: ResourceArc, attrs: InodeAttributes) -> Inode {
        Inode::new(&RAMFS_INODE_OPS)
            .with_attributes(attrs)
            .with_private(Arc::new(RamfsInode { res }))
    }
}

fn ramfs_inode_open(inode: &InodeArc, result: fs::Sender<ResourceArc>) {
    let private = inode.private::<RamfsInode>().unwrap();
    result.send(Ok(private.res.clone()))
}

static RAMFS_INODE_OPS: InodeOps = InodeOps {
    open: ramfs_inode_open,
};

struct RamfsSuperblock {
    root: InodeArc,
}

impl RamfsSuperblock {
    pub fn new() -> Superblock {
        Superblock::new(&RAMFS_SUPERBLOCK_OPS).with_private(Arc::new(RamfsSuperblock {
            root: InodeArc::new_cyclic(|inode| {
                RamfsInode::new(
                    RamfsResource::dir(inode.clone()).into(),
                    InodeAttributes::root(InodeMode::DEFAULT_DIR),
                )
            }),
        }))
    }
}

fn ramfs_sp_root(sp: &SuperblockArc, result: fs::Sender<InodeArc>) {
    result.send_with(|| {
        let private = sp.private::<RamfsSuperblock>().unwrap();
        Ok(private.root.clone())
    });
}

static RAMFS_SUPERBLOCK_OPS: SuperblockOps = SuperblockOps {
    root: ramfs_sp_root,
};

fn ramfs_mount(
    _: &Filesystem,
    blkdev: Option<Arc<dyn BlockDevice>>,
    result: fs::Sender<SuperblockArc>,
) {
    result.send_with(|| {
        if blkdev.is_some() {
            return Err(fs::Error::InvalidArgument);
        }
        Ok(RamfsSuperblock::new().into())
    });
}
static RAMFS_OPS: FilesystemOps = FilesystemOps { mount: ramfs_mount };
static RAMFS: Filesystem = Filesystem::new("ramfs", &RAMFS_OPS);

fn ramfs_init(module: Module) {
    register_filesystem(&RAMFS, module).expect("ramfs name conflict");
}

fn ramfs_fini() {
    assert!(
        unregister_filesystem(&RAMFS).is_ok(),
        "ramfs should be registered"
    );
}

module_decl!(ModuleDecl::new("ramfs").with_init_fini(ramfs_init, ramfs_fini));
