#![no_std]
#![feature(try_blocks)]

extern crate alloc;

use alloc::sync::Arc;
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
    ResourceArc, ResourceOps,
};

struct RamfsFile {}

impl RamfsFile {
    fn new() -> File {
        todo!()
    }
}

static RAMFS_FILE_OPS: FileOps = FileOps {
    read: |_, _, _| todo!(),
    write: |_, _, _| todo!(),
};

struct RamfsDir {}

impl RamfsDir {
    fn new() -> Directory {
        todo!()
    }
}

fn ramfs_dir_list(
    _dir: &DirectoryArc,
    buf: BufOwn<MaybeUninit<DirectoryEntry>>,
    result: fs::Sender<(usize, BufOwn<MaybeUninit<DirectoryEntry>>)>,
) {
    result.send_with(move || Ok((0, buf)))
}

fn ramfs_dir_mkfile(
    _dir: &DirectoryArc,
    _name: &str,
    _attrs: InodeAttributes,
    _result: fs::Sender<FileArc>,
) {
    todo!()
}

fn ramfs_dir_mkdir(
    _dir: &DirectoryArc,
    _name: &str,
    _attrs: InodeAttributes,
    _result: fs::Sender<DirectoryArc>,
) {
    todo!()
}

static RAMFS_DIR_OPS: DirectoryOps = DirectoryOps {
    list: ramfs_dir_list,
    mkfile: Some(ramfs_dir_mkfile),
    mkdir: Some(ramfs_dir_mkdir),
};

enum RamfsResource {
    File(FileArc),
    Dir(DirectoryArc),
}

impl RamfsResource {
    pub fn file(inode: InodeWeak, parent: InodeWeak) -> Resource {
        let file = File::new(&RAMFS_FILE_OPS).with_private(Arc::new(RamfsFile {}));
        Resource::new(&RAMFS_RES_OPS)
            .with_inode(inode)
            .with_parent(parent)
            .with_private(Arc::new(RamfsResource::File(file.into())))
    }

    pub fn dir(inode: InodeWeak, parent: Option<InodeWeak>) -> Resource {
        let dir = Directory::new(&RAMFS_DIR_OPS).with_private(Arc::new(RamfsDir {}));
        let mut res = Resource::new(&RAMFS_RES_OPS)
            .with_inode(inode)
            .with_private(Arc::new(RamfsResource::Dir(dir.into())));
        if let Some(parent) = parent {
            res = res.with_parent(parent);
        }
        res
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
                    RamfsResource::dir(inode.clone(), None).into(),
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
