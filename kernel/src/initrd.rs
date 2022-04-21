use crate::async_::oneshot::call_with_sender;
use crate::fs::with_filesystem;

const RAMFS_FS_NAME: &'static str = "ramfs";

fn load_initrd_modules() {}

pub async fn load_initrd(_initrd: &[u8]) {
    let sp = with_filesystem(RAMFS_FS_NAME, |fs| {
        call_with_sender!((fs.ops.mount)(fs, None))
    })
    .expect("ramfs not found")
    .await
    .expect("Could not mount ramfs");

    let _root = (*sp).async_root().await.expect("Could not get ramfs root");
}
