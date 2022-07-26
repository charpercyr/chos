use alloc::borrow::Cow;
use core::future::Future;
use core::mem::{replace, MaybeUninit};

use chos_lib::mm::VAddr;
use chos_lib::pool::{iarc_adapter_weak, IArc, IArcCountWeak, IWeak};
use chos_lib::sync::Spinlock;

use crate::async_::oneshot::call_with_sender;
use crate::fs::buf::{Buf, BufOwn};
use crate::fs::{self, InodeArc, InodeAttributes, InodeWeak};
use crate::mm::slab::object_pool;
use crate::private_project_impl;
use crate::util::{private_impl, Private};

pub struct ResourceOps {
    pub file: fn(&ResourceArc) -> Option<FileArc>,
    pub dir: fn(&ResourceArc) -> Option<DirectoryArc>,
}

pub struct Resource {
    count: IArcCountWeak,
    ops: &'static ResourceOps,
    pub inode: Option<InodeWeak>,
    private: Option<Private>,
}
iarc_adapter_weak!(Resource: count);
object_pool!(pub struct ResourcePool : Resource);
pub type ResourceArc = IArc<Resource, ResourcePool>;
pub type ResourceWeak = IWeak<Resource, ResourcePool>;

impl Resource {
    pub const fn new(ops: &'static ResourceOps) -> Self {
        Self {
            count: IArcCountWeak::new(),
            ops,
            inode: None,
            private: None,
        }
    }

    pub fn with_private(self, private: Private) -> Self {
        Self {
            private: Some(private),
            ..self
        }
    }

    pub fn with_inode(self, inode: InodeWeak) -> Self {
        Self {
            inode: Some(inode),
            ..self
        }
    }

    pub fn inode(&self) -> Option<InodeArc> {
        self.inode.as_ref()?.upgrade()
    }

    pub fn file(self: &ResourceArc) -> Option<FileArc> {
        (self.ops.file)(self)
    }

    pub fn dir<'a>(self: &ResourceArc) -> Option<DirectoryArc> {
        (self.ops.dir)(self)
    }

    private_impl!(private);
}

pub struct FileOps {
    pub read: fn(&FileArc, u64, BufOwn<u8>, fs::Sender<(usize, BufOwn<u8>)>),
    pub write: fn(&FileArc, u64, BufOwn<u8>, fs::Sender<(usize, BufOwn<u8>)>),
}

pub struct FileMut {
    private: Option<Private>,
}

impl FileMut {
    private_impl!(private);
}

pub struct File {
    count: IArcCountWeak,
    pub resource: ResourceWeak,
    ops: &'static FileOps,
    pub file_mut: Spinlock<FileMut>,
}
iarc_adapter_weak!(File: count);
object_pool!(pub struct FilePool : File);
pub type FileArc = IArc<File, FilePool>;
pub type FileWeak = IWeak<File, FilePool>;

impl File {
    pub const fn new(ops: &'static FileOps, resource: ResourceWeak) -> Self {
        Self {
            count: IArcCountWeak::new(),
            resource,
            ops,
            file_mut: Spinlock::new(FileMut { private: None }),
        }
    }

    pub fn with_private(mut self, private: Private) -> Self {
        self.file_mut.get_mut().private = Some(private);
        self
    }

    pub fn resource(&self) -> Option<ResourceArc> {
        self.resource.upgrade()
    }

    pub fn inode(&self) -> Option<InodeArc> {
        self.resource()?.inode()
    }

    pub fn read(
        self: &FileArc,
        offset: u64,
        buf: BufOwn<u8>,
        result: fs::Sender<(usize, BufOwn<u8>)>,
    ) {
        (self.ops.read)(self, offset, buf, result)
    }

    pub async fn async_read(
        self: &FileArc,
        offset: u64,
        buf: BufOwn<u8>,
    ) -> fs::Result<(usize, BufOwn<u8>)> {
        call_with_sender!((Self::read)(self, offset, buf)).await
    }

    pub async fn async_read_all(
        self: &FileArc,
        mut offset: u64,
        mut buf: &mut [u8],
    ) -> fs::Result<()> {
        while !buf.is_empty() {
            let buf_own = unsafe { BufOwn::new_single(Buf::from_slice_mut(buf)) };
            let (read, _) = self.async_read(offset, buf_own).await?;
            offset += read as u64;
            buf = &mut buf[read..];
        }
        Ok(())
    }

    pub fn write(
        self: &FileArc,
        offset: u64,
        buf: BufOwn<u8>,
        result: fs::Sender<(usize, BufOwn<u8>)>,
    ) {
        (self.ops.write)(self, offset, buf, result)
    }

    pub async fn async_write(
        self: &FileArc,
        offset: u64,
        buf: BufOwn<u8>,
    ) -> fs::Result<(usize, BufOwn<u8>)> {
        call_with_sender!((Self::write)(self, offset, buf)).await
    }

    pub async fn async_write_all(
        self: &FileArc,
        mut offset: u64,
        mut buf: &[u8],
    ) -> fs::Result<()> {
        while !buf.is_empty() {
            let buf_own = unsafe { BufOwn::new_single(Buf::from_slice(buf)) };
            let (written, _) = self.async_write(offset, buf_own).await?;
            offset += written as u64;
            buf = &buf[written..];
        }
        Ok(())
    }

    private_project_impl!(file_mut: FileMut => private);
}

#[derive(Clone)]
pub struct DirectoryEntry {
    pub name: Cow<'static, str>,
    pub inode: InodeArc,
}

pub struct DirectoryOps {
    pub list_iter: fn(
        &DirectoryArc,
        usize,
        BufOwn<DirectoryEntry>,
        fs::Sender<(usize, BufOwn<DirectoryEntry>)>,
    ),
    pub mkfile: Option<fn(&DirectoryArc, &str, InodeAttributes, fs::Sender<FileArc>)>,
    pub mkdir: Option<fn(&DirectoryArc, &str, InodeAttributes, fs::Sender<DirectoryArc>)>,
}

pub struct DirectoryMut {
    private: Option<Private>,
}

impl DirectoryMut {
    private_impl!(private);
}

pub struct Directory {
    count: IArcCountWeak,
    pub resource: ResourceWeak,
    ops: &'static DirectoryOps,
    pub dir_mut: Spinlock<DirectoryMut>,
}
iarc_adapter_weak!(Directory: count);
object_pool!(pub struct DirectoryPool : Directory);
pub type DirectoryArc = IArc<Directory, DirectoryPool>;
pub type DirectoryWeak = IWeak<Directory, DirectoryPool>;

impl Directory {
    pub fn new(ops: &'static DirectoryOps, resource: ResourceWeak) -> Self {
        Self {
            count: IArcCountWeak::new(),
            resource,
            ops,
            dir_mut: Spinlock::new(DirectoryMut { private: None }),
        }
    }

    pub fn with_private(mut self, private: Private) -> Self {
        self.dir_mut.get_mut().private = Some(private);
        self
    }

    pub fn resource(&self) -> Option<ResourceArc> {
        self.resource.upgrade()
    }

    pub fn inode(&self) -> Option<InodeArc> {
        self.resource()?.inode()
    }

    pub fn list_iter(
        self: &DirectoryArc,
        idx: usize,
        buf: BufOwn<DirectoryEntry>,
        result: fs::Sender<(usize, BufOwn<DirectoryEntry>)>,
    ) {
        (self.ops.list_iter)(self, idx, buf, result)
    }

    pub async fn async_list_iter(
        self: &DirectoryArc,
        idx: usize,
        buf: BufOwn<DirectoryEntry>,
    ) -> fs::Result<(usize, BufOwn<DirectoryEntry>)> {
        call_with_sender!((Self::list_iter)(self, idx, buf)).await
    }

    pub async fn async_list<R>(
        self: &DirectoryArc,
        mut callback: impl FnMut(DirectoryEntry) -> Option<R>,
    ) -> fs::Result<Option<R>> {
        let mut buf: [_; 16] = MaybeUninit::uninit_array();
        let mut idx = 0;
        loop {
            let buf_own = unsafe { BufOwn::new_single(Buf::from_slice_uninit_mut(&mut buf)) };
            let (len, _) = self.async_list_iter(idx, buf_own).await?;
            for i in 0..len {
                if let Some(r) =
                    callback(unsafe { replace(&mut buf[i], MaybeUninit::uninit()).assume_init() })
                {
                    return Ok(Some(r));
                }
            }
            if len < buf.len() {
                break;
            }
            idx += len;
        }
        Ok(None)
    }

    pub async fn async_list_fut<R, F: Future<Output = Option<R>>>(
        self: &DirectoryArc,
        mut callback: impl FnMut(DirectoryEntry) -> F,
    ) -> fs::Result<Option<R>> {
        let mut buf: [_; 16] = MaybeUninit::uninit_array();
        let mut idx = 0;
        loop {
            let buf_own = unsafe { BufOwn::new_single(Buf::from_slice_uninit_mut(&mut buf)) };
            let (len, _) = self.async_list_iter(idx, buf_own).await?;
            for i in 0..len {
                if let Some(r) =
                    callback(unsafe { replace(&mut buf[i], MaybeUninit::uninit()).assume_init() })
                        .await
                {
                    return Ok(Some(r));
                }
            }
            if len < buf.len() {
                break;
            }
            idx += len;
        }
        Ok(None)
    }

    pub fn mkfile(
        self: &DirectoryArc,
        name: &str,
        attrs: InodeAttributes,
        result: fs::Sender<FileArc>,
    ) {
        if let Some(mkfile) = self.ops.mkfile {
            mkfile(self, name, attrs, result)
        } else {
            result.send(Err(fs::Error::NotSupported));
        }
    }

    pub async fn async_mkfile(
        self: &DirectoryArc,
        name: &str,
        attrs: InodeAttributes,
    ) -> fs::Result<FileArc> {
        call_with_sender!((Self::mkfile)(self, name, attrs)).await
    }

    pub fn mkdir(
        self: &DirectoryArc,
        name: &str,
        attrs: InodeAttributes,
        result: fs::Sender<DirectoryArc>,
    ) {
        if let Some(mkdir) = self.ops.mkdir {
            mkdir(self, name, attrs, result)
        } else {
            result.send(Err(fs::Error::NotSupported));
        }
    }

    pub async fn async_mkdir(
        self: &DirectoryArc,
        name: &str,
        attrs: InodeAttributes,
    ) -> fs::Result<DirectoryArc> {
        call_with_sender!((Self::mkdir)(self, name, attrs)).await
    }

    private_project_impl!(dir_mut: DirectoryMut => private);
}

pub struct MemoryOps {
    pub map: fn(fs::Sender<VAddr>),
    pub unmap: fn(fs::Sender<()>),
}

pub struct MemoryMut {
    private: Option<Private>,
}

impl MemoryMut {
    private_impl!(private);
}

pub struct Memory {
    count: IArcCountWeak,
    pub resource: ResourceWeak,
    ops: &'static MemoryOps,
    pub mem_mut: Spinlock<MemoryMut>,
}

impl Memory {
    pub const fn new(ops: &'static MemoryOps, resource: ResourceWeak) -> Self {
        Self {
            count: IArcCountWeak::new(),
            resource,
            ops,
            mem_mut: Spinlock::new(MemoryMut { private: None }),
        }
    }

    pub fn with_private(mut self, private: Private) -> Self {
        self.mem_mut.get_mut().private = Some(private);
        self
    }

    private_project_impl!(mem_mut: MemoryMut => private);
}
