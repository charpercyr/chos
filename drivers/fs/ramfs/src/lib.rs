#![no_std]
#![feature(allocator_api)]
#![feature(try_blocks)]

extern crate alloc;

use alloc::boxed::Box;
use alloc::sync::Arc;
use alloc::vec::Vec;

use chos::driver::block::BlockDevice;
use chos::fs::buf::BufOwn;
use chos::fs::{
    self, register_filesystem, unregister_filesystem, Filesystem, FilesystemOps, Inode, InodeArc,
    InodeAttributes, InodeMode, InodeOps, InodeWeak, Superblock, SuperblockArc, SuperblockOps,
};
use chos::mm::slab::object_pool;
use chos::module::{module_decl, Module, ModuleDecl};
use chos::resource::{
    Directory, DirectoryArc, DirectoryEntry, DirectoryOps, File, FileArc, FileOps, Resource,
    ResourceArc, ResourceOps, ResourceWeak,
};
use chos_lib::arch::mm::DefaultFrameSize;
use chos_lib::boxed::try_new_boxed_array;
use chos_lib::intrusive_adapter;
use chos_lib::mm::FrameSize;
use chos_lib::pool::PoolBox;
use intrusive_collections::linked_list;

const FILE_BLOCK_SIZE: u64 = DefaultFrameSize::PAGE_SIZE;
const FILE_BLOCK_MASK: u64 = DefaultFrameSize::PAGE_MASK;

struct RamfsFileBlock {
    link: linked_list::AtomicLink,
    block: Box<[u8; FILE_BLOCK_SIZE as usize]>,
    offset: u64,
}
object_pool!(struct RamfsFileBlockPool : RamfsFileBlock);
type RamfsFileBlockBox = PoolBox<RamfsFileBlock, RamfsFileBlockPool>;
intrusive_adapter!(RamfsFileBlockAdapter = RamfsFileBlockBox : RamfsFileBlock { link: linked_list::AtomicLink });

enum BlockResult<'a> {
    Found(linked_list::CursorMut<'a, RamfsFileBlockAdapter>),
    Previous(linked_list::CursorMut<'a, RamfsFileBlockAdapter>),
}

struct RamfsFile {
    blocks: linked_list::LinkedList<RamfsFileBlockAdapter>,
    len: usize,
}

impl RamfsFile {
    fn new(resource: ResourceWeak) -> File {
        File::new(&RAMFS_FILE_OPS, resource).with_private(Box::new(RamfsFile {
            blocks: linked_list::LinkedList::new(RamfsFileBlockAdapter::new()),
            len: 0,
        }))
    }

    fn write(&mut self, mut offset: u64, buf_in: &BufOwn<u8>) -> fs::Result<usize> {
        let mut reader = buf_in.reader();
        let mut cursor = self.find_or_alloc_block(offset)?;
        let mut written = 0;
        loop {
            let block_start = (offset % FILE_BLOCK_SIZE) as usize;
            let block_size = FILE_BLOCK_SIZE as usize - block_start;
            let block_written =
                reader.read(unsafe { &mut cursor.get_mut().unwrap().block[block_start..] });
            written += block_written;
            if block_written == block_size as usize {
                offset += block_size as u64;
                cursor = Self::find_or_alloc_block_starting_from(cursor, offset)?;
            } else {
                break Ok(written)
            }
        }
    }

    fn read(&mut self, mut offset: u64, buf_out: &mut BufOwn<u8>) -> fs::Result<usize> {
        let mut read = 0;
        let mut writer = buf_out.writer();
        let mut cursor = self.blocks.front_mut();
        loop {
            let block_start = (offset % FILE_BLOCK_SIZE) as usize;
            let block_size = FILE_BLOCK_SIZE as usize - block_start;
            let block_read;
            match Self::find_block_starting_from(cursor, offset) {
                BlockResult::Found(block) => {
                    block_read = writer.write(&block.get().unwrap().block[block_start..]);
                    cursor = block;
                },
                BlockResult::Previous(prev) => {
                    block_read = writer.write_bytes(0x00, block_size);
                    cursor = prev;
                },
            }
            read += block_read;
            if block_read == block_size as usize {
                offset += block_size as u64;
            } else {
                break Ok(read)
            }
        }
    }

    fn find_block_starting_from<'a>(
        mut cur: linked_list::CursorMut<'a, RamfsFileBlockAdapter>,
        offset: u64,
    ) -> BlockResult<'a> {
        while !cur.is_null() {
            let block = cur.get().unwrap();
            if offset >= block.offset && offset < block.offset + FILE_BLOCK_SIZE {
                return BlockResult::Found(cur);
            } else if offset > block.offset {
                break;
            }
            cur.move_next();
        }
        cur.move_prev();
        BlockResult::Previous(cur)
    }

    fn find_block(&mut self, offset: u64) -> BlockResult<'_> {
        Self::find_block_starting_from(self.blocks.front_mut(), offset)
    }

    fn find_or_alloc_block_starting_from<'a>(
        cur: linked_list::CursorMut<'a, RamfsFileBlockAdapter>,
        offset: u64,
    ) -> fs::Result<linked_list::CursorMut<'a, RamfsFileBlockAdapter>> {
        let mut prev = match Self::find_block_starting_from(cur, offset) {
            BlockResult::Found(cur) => return Ok(cur),
            BlockResult::Previous(prev) => prev,
        };
        let page_offset = offset & FILE_BLOCK_MASK;
        let block = try_new_boxed_array().map_err(|_| fs::Error::AllocError)?;
        let block = PoolBox::try_new(RamfsFileBlock {
            link: linked_list::AtomicLink::new(),
            offset: page_offset,
            block,
        })
        .map_err(|_| fs::Error::AllocError)?;
        prev.insert_after(block);
        prev.move_next();
        Ok(prev)
    }

    fn find_or_alloc_block(
        &mut self,
        offset: u64,
    ) -> fs::Result<linked_list::CursorMut<RamfsFileBlockAdapter>> {
        Self::find_or_alloc_block_starting_from(self.blocks.front_mut(), offset)
    }
}

fn ramfs_file_read(
    file: &FileArc,
    offset: u64,
    mut buf: BufOwn<u8>,
    result: fs::Sender<(usize, BufOwn<u8>)>,
) {
    result.send_with(move || {
        let mut private = file.lock_private::<RamfsFile>().unwrap();
        let read = private.read(offset, &mut buf)?;
        Ok((read, buf))
    })
}

fn ramfs_file_write(
    file: &FileArc,
    offset: u64,
    buf: BufOwn<u8>,
    result: fs::Sender<(usize, BufOwn<u8>)>,
) {
    result.send_with(move || {
        let mut private = file.lock_private::<RamfsFile>().unwrap();
        let written = private.write(offset, &buf)?;
        Ok((written, buf))
    })
}

static RAMFS_FILE_OPS: FileOps = FileOps {
    read: ramfs_file_read,
    write: ramfs_file_write,
};

struct RamfsDir {
    children: Vec<DirectoryEntry>,
}

impl RamfsDir {
    fn new(resource: ResourceWeak) -> Directory {
        Directory::new(&RAMFS_DIR_OPS, resource).with_private(Box::new(RamfsDir {
            children: Vec::new(),
        }))
    }
}

fn ramfs_dir_list(
    dir: &DirectoryArc,
    idx: usize,
    mut buf: BufOwn<DirectoryEntry>,
    result: fs::Sender<(usize, BufOwn<DirectoryEntry>)>,
) {
    result.send_with(|| {
        let private = dir.lock_private::<RamfsDir>().unwrap();
        let mut writer = buf.writer();
        let mut written = 0;
        let inode = dir
            .resource
            .upgrade()
            .unwrap()
            .inode
            .as_ref()
            .unwrap()
            .upgrade()
            .unwrap();
        if idx <= 0 {
            if let Err(_) = writer.write_one(DirectoryEntry {
                name: ".".into(),
                inode: inode.clone(),
            }) {
                return Ok((0, buf));
            }
            written += 1;
        }
        if idx <= 1 {
            if let Err(_) = writer.write_one(DirectoryEntry {
                name: "..".into(),
                inode: inode
                    .parent
                    .as_ref()
                    .and_then(|inode| inode.upgrade())
                    .unwrap_or_else(|| inode),
            }) {
                return Ok((written, buf));
            }
            written += 1;
        }
        let children = if idx < 2 {
            &private.children[..]
        } else {
            &private.children[idx - 2..]
        };
        written += writer.write_iter(children.iter().cloned());
        Ok((written, buf))
    })
}

fn ramfs_dir_mkfile(
    dir: &DirectoryArc,
    name: &str,
    attrs: InodeAttributes,
    result: fs::Sender<FileArc>,
) {
    let mut private = dir.lock_private::<RamfsDir>().unwrap();
    let inode =
        InodeArc::new_cyclic(|inode| RamfsInode::new(RamfsResource::file(inode.clone()), attrs));
    let file = inode.private::<RamfsInode>().unwrap().res.file().unwrap();
    private.children.push(DirectoryEntry {
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
    let mut private = dir.lock_private::<RamfsDir>().unwrap();
    let inode =
        InodeArc::new_cyclic(|inode| RamfsInode::new(RamfsResource::dir(inode.clone()), attrs));
    let file = inode.private::<RamfsInode>().unwrap().res.dir().unwrap();
    private.children.push(DirectoryEntry {
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
                .with_private(Box::new(RamfsResource::File(
                    RamfsFile::new(res.clone()).into(),
                )))
        })
    }

    pub fn dir(inode: InodeWeak) -> ResourceArc {
        ResourceArc::new_cyclic(|res| {
            Resource::new(&RAMFS_RES_OPS)
                .with_inode(inode)
                .with_private(Box::new(RamfsResource::Dir(
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
            .with_private(Box::new(RamfsInode { res }))
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
        Superblock::new(&RAMFS_SUPERBLOCK_OPS).with_private(Box::new(RamfsSuperblock {
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
    let sp = sp.clone();
    result.send_with(move || {
        let private = sp.lock_private::<RamfsSuperblock>().unwrap();
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
