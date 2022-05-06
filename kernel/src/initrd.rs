use chos_lib::log::println;
use chos_lib::tar::Tar;

use crate::async_::oneshot::call_with_sender;
use crate::fs::{with_filesystem, Filesystem};

const RAMFS_FS_NAME: &'static str = "ramfs";

fn load_initrd_modules() {}

pub async fn load_initrd(initrd: &[u8]) {
    let _initrd = Tar::new(initrd).expect("Initrd not a valid tar file");

    let sp = with_filesystem(RAMFS_FS_NAME, |fs| {
        call_with_sender!((Filesystem::mount)(fs, None))
    })
    .expect("ramfs not found")
    .await
    .expect("Could not mount ramfs");

    let root = sp.async_root().await.expect("Could not get ramfs root");
    let root = root.async_open().await.expect("Could not open ramfs root");
    let _root = root.dir().expect("Root should be a directory");

    println!("Here");
}
