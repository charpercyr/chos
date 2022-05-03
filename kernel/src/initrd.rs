
use core::mem::MaybeUninit;

use chos_lib::log::println;

use crate::async_::oneshot::{call_with_sender};
use crate::fs::{with_filesystem, Filesystem};

const RAMFS_FS_NAME: &'static str = "ramfs";

fn load_initrd_modules() {}

pub async fn load_initrd(_initrd: &[u8]) {
    let sp = with_filesystem(RAMFS_FS_NAME, |fs| {
        call_with_sender!((Filesystem::mount)(fs, None))
    })
    .expect("ramfs not found")
    .await
    .expect("Could not mount ramfs");

    let root = sp.async_root().await.expect("Could not get ramfs root");
    let root = root.async_open().await.expect("Could not open ramfs root");
    let root = root.dir().expect("Root should be a directory");

    let mut direntries = MaybeUninit::uninit_array::<64>();
    let entries = root.async_list(&mut direntries).await.unwrap();

    println!("Entries");
    for e in entries {
        println!("{}", e.name);
    }
}
