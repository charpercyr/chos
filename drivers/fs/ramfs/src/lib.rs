#![no_std]
#![feature(try_blocks)]

extern crate alloc;

use alloc::boxed::Box;
use alloc::sync::Arc;

use chos::driver::block::BlockDevice;
use chos::fs::{
    self, register_filesystem, unregister_filesystem, Filesystem, FilesystemOps,
    Superblock,
};
use chos::module::{module_decl, Module, ModuleDecl};

struct RamfsSuperblock {
}

impl Superblock for RamfsSuperblock {
    fn root(&self, _result: fs::Sender<Arc<dyn fs::Inode>>) {
        todo!()
    }
}

fn ramfs_mount(
    _: &Filesystem,
    blkdev: Option<Arc<dyn BlockDevice>>,
    result: fs::Sender<Box<dyn Superblock>>,
) {
    result.send_with(move || {
        if blkdev.is_some() {
            return Err(fs::Error::InvalidArgument);
        }
        todo!()
    })
}
static RAMFS: Filesystem = Filesystem::new("ramfs", &FilesystemOps { mount: ramfs_mount });

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
