use chos_lib::tar::raw::EntryType;
use chos_lib::tar::Tar;

use crate::async_::oneshot::call_with_sender;
use crate::fs::buf::BufOwn;
use crate::fs::path::{Component, Path};
use crate::fs::{with_filesystem, Filesystem, InodeAttributes, InodeMode};
use crate::resource::DirectoryArc;

const RAMFS_FS_NAME: &'static str = "ramfs";

async fn create_file(path: &Path, root: &DirectoryArc, mut contents: &[u8]) {
    let filename = path.file_name().expect("Should have a file name");
    let dirname = path.parent().unwrap_or(Path::new("."));
    let mut dir = root.clone();
    for c in dirname.components() {
        match c {
            Component::CurDir | Component::RootDir => (),
            Component::ParentDir => panic!("Not supported"),
            Component::Normal(name) => {
                if let Some(direntry) = dir
                    .async_list(|entry| (entry.name == name).then(|| entry))
                    .await
                    .unwrap()
                {
                    let res = direntry.inode.async_open().await.unwrap();
                    dir = res.dir().unwrap();
                } else {
                    dir = dir
                        .async_mkdir(name, InodeAttributes::root(InodeMode::DEFAULT_DIR))
                        .await
                        .unwrap();
                }
            }
        }
    }

    let file = dir
        .async_mkfile(filename, InodeAttributes::root(InodeMode::DEFAULT_FILE))
        .await
        .unwrap();

    let mut offset = 0;
    while !contents.is_empty() {
        let buf = unsafe { BufOwn::from_slice(contents) };
        let (written, _) = file.async_write(offset, buf).await.unwrap();
        offset += written;
        contents = &contents[written..];
    }
}

pub async fn load_initrd(initrd: &[u8]) {
    let initrd = Tar::new(initrd).expect("Initrd not a valid tar file");

    let sp = with_filesystem(RAMFS_FS_NAME, |fs| {
        call_with_sender!((Filesystem::mount)(fs, None))
    })
    .expect("ramfs not found")
    .await
    .expect("Could not mount ramfs");

    let root = sp.async_root().await.expect("Could not get ramfs root");
    let root = root.async_open().await.expect("Could not open ramfs root");
    let root = root.dir().expect("Root should be a directory");

    for file in &initrd {
        if file.typ() == EntryType::File {
            let filename = file.name_merged();
            let path = Path::new(&filename);
            create_file(path, &root, file.contents()).await;
        }
    }
}
