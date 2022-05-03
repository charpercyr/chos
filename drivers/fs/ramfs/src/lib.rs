#![no_std]
#![feature(try_blocks)]

extern crate alloc;

use alloc::sync::Arc;
use core::mem::MaybeUninit;

use chos::driver::block::BlockDevice;
use chos::fs::buf::BufOwn;
use chos::fs::{
    self, register_filesystem, unregister_filesystem, Filesystem, FilesystemOps, Inode, InodeArc,
    InodeOps, Superblock, SuperblockArc, SuperblockOps,
};
use chos::module::{module_decl, Module, ModuleDecl};
use chos::resource::{
    Directory, DirectoryArc, DirectoryEntry, DirectoryOps, File, FileArc, FileOps, Resource,
    ResourceArc, ResourceOps,
};

struct RamfsFile {}

impl RamfsFile {
    fn new() -> FileArc {
        File::with_private(&RAMFS_FILE_OPS, Arc::new(RamfsFile {}))
    }
}

static RAMFS_FILE_OPS: FileOps = FileOps {
    read: |_, _, _| todo!(),
    write: |_, _, _| todo!(),
};

struct RamfsDir {}

impl RamfsDir {
    fn new() -> DirectoryArc {
        Directory::with_private(&RAMFS_DIR_OPS, Arc::new(RamfsDir {}))
    }
}

fn ramfs_dir_list(
    _dir: &DirectoryArc,
    buf: BufOwn<MaybeUninit<DirectoryEntry>>,
    result: fs::Sender<(usize, BufOwn<MaybeUninit<DirectoryEntry>>)>,
) {
    result.send_with(move || {
        Ok((0, buf))
    })
}

static RAMFS_DIR_OPS: DirectoryOps = DirectoryOps {
    list: ramfs_dir_list,
    mkfile: None,
    mkdir: None,
};

enum RamfsResource {
    None,
    File(FileArc),
    Dir(DirectoryArc),
}

impl RamfsResource {
    pub fn none() -> ResourceArc {
        Resource::with_private(&RAMFS_RES_OPS, Arc::new(Self::None))
    }

    pub fn file() -> ResourceArc {
        Resource::with_private(&RAMFS_RES_OPS, Arc::new(Self::File(RamfsFile::new())))
    }

    pub fn dir() -> ResourceArc {
        Resource::with_private(&RAMFS_RES_OPS, Arc::new(Self::Dir(RamfsDir::new())))
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
    pub fn new(res: ResourceArc) -> InodeArc {
        Inode::with_private(&RAMFS_INODE_OPS, Arc::new(Self { res }))
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
    pub fn new() -> SuperblockArc {
        Superblock::with_private(
            &RAMFS_SUPERBLOCK_OPS,
            Arc::new(Self {
                root: RamfsInode::new(RamfsResource::dir()),
            }),
        )
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
        Ok(RamfsSuperblock::new())
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
