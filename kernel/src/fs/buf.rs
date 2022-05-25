use core::mem::{replace, MaybeUninit};
use core::ptr::{copy_nonoverlapping, write_bytes, NonNull};

use chos_lib::mem::{maybe_uninit_init_slice, maybe_uninit_init_slice_mut};
use chos_lib::pod::Pod;

pub struct Buf<T> {
    buf: NonNull<[MaybeUninit<T>]>,
}
unsafe impl<T: Send> Send for Buf<T> {}
unsafe impl<T: Sync> Sync for Buf<T> {}

impl<T> Buf<T> {
    pub unsafe fn from_raw_parts_uninit(ptr: NonNull<MaybeUninit<T>>, len: usize) -> Self {
        Self {
            buf: NonNull::slice_from_raw_parts(ptr, len),
        }
    }

    pub unsafe fn from_slice_uninit(buf: &[MaybeUninit<T>]) -> Self {
        Self { buf: buf.into() }
    }

    pub unsafe fn from_slice_uninit_mut(buf: &mut [MaybeUninit<T>]) -> Self {
        Self { buf: buf.into() }
    }

    pub fn as_slice_uninit(&self) -> &[MaybeUninit<T>] {
        unsafe { self.buf.as_ref() }
    }

    pub fn as_slice_uninit_mut(&mut self) -> &mut [MaybeUninit<T>] {
        unsafe { self.buf.as_mut() }
    }
}

impl<T: Pod> Buf<T> {
    pub unsafe fn from_raw_parts(ptr: NonNull<T>, len: usize) -> Self {
        Self::from_raw_parts_uninit(ptr.cast(), len)
    }

    pub unsafe fn from_slice(buf: &[T]) -> Self {
        Self::from_slice_uninit(maybe_uninit_init_slice(buf))
    }

    pub unsafe fn from_slice_mut(buf: &mut [T]) -> Self {
        Self::from_slice_uninit(maybe_uninit_init_slice_mut(buf))
    }

    pub fn as_slice(&self) -> &[T] {
        unsafe { MaybeUninit::slice_assume_init_ref(self.as_slice_uninit()) }
    }

    pub fn as_slice_mut(&mut self) -> &mut [T] {
        unsafe { MaybeUninit::slice_assume_init_mut(self.as_slice_uninit_mut()) }
    }
}

pub enum BufOwn<T> {
    Single(Buf<T>),
}

impl<T> BufOwn<T> {
    pub const fn new_single(buf: Buf<T>) -> Self {
        Self::Single(buf)
    }

    pub const fn single(&self) -> Option<&Buf<T>> {
        match self {
            Self::Single(buf) => Some(buf),
        }
    }

    pub const fn single_mut(&mut self) -> Option<&mut Buf<T>> {
        match self {
            Self::Single(buf) => Some(buf),
        }
    }

    pub fn writer(&mut self) -> BufWriter<'_, T> {
        let buf = match self {
            Self::Single(buf) => buf.as_slice_uninit_mut().into(),
        };
        BufWriter {
            buf: self,
            cur_buf: Some(buf),
            buf_idx: 0,
        }
    }

    pub fn reader(&self) -> BufReader<'_, T> {
        let buf = match self {
            Self::Single(buf) => buf.as_slice_uninit(),
        };
        BufReader {
            buf: self,
            cur_buf: Some(buf),
            buf_idx: 0,
        }
    }

    pub fn len(&self) -> usize {
        match self {
            Self::Single(buf) => unsafe { buf.buf.as_ref().len() },
        }
    }
}

unsafe fn bitcopy_nonoverlapping<T>(
    dst: &mut [MaybeUninit<T>],
    src: &[MaybeUninit<T>],
) -> BufferState {
    if dst.len() > src.len() {
        copy_nonoverlapping(src.as_ptr(), dst.as_mut_ptr(), src.len());
        BufferState::SrcEmpty
    } else if dst.len() == src.len() {
        copy_nonoverlapping(src.as_ptr(), dst.as_mut_ptr(), src.len());
        BufferState::SameSize
    } else {
        copy_nonoverlapping(src.as_ptr(), dst.as_mut_ptr(), dst.len());
        BufferState::DstFull
    }
}

enum BufferState {
    DstFull,
    SrcEmpty,
    SameSize,
}

pub struct BufWriter<'a, T> {
    buf: &'a mut BufOwn<T>,
    cur_buf: Option<NonNull<[MaybeUninit<T>]>>, // &'a mut [T]
    buf_idx: usize,
}

impl<T> BufWriter<'_, T> {
    pub unsafe fn write_uninit(&mut self, src: &mut [MaybeUninit<T>]) -> usize {
        let written = self.write_bitcopy(src);
        #[cfg(debug_assertions)]
        write_bytes(src.as_mut_ptr(), 0xcc, written);
        written
    }

    pub fn write_one(&mut self, value: T) -> Result<(), T> {
        let mut array = [MaybeUninit::new(value)];
        let written = unsafe { self.write_uninit(&mut array) };
        match written {
            0 => Err(unsafe { replace(&mut array[0], MaybeUninit::uninit()).assume_init() }),
            1 => Ok(()),
            _ => panic!("Should not have written more than 1"),
        }
    }

    pub fn write_iter(&mut self, iter: impl IntoIterator<Item = T>) -> usize {
        let mut written = 0;
        for item in iter {
            if let Err(_) = self.write_one(item) {
                break;
            }
            written += 1;
        }
        written
    }

    unsafe fn write_bitcopy(&mut self, mut src: &[MaybeUninit<T>]) -> usize {
        let mut written = 0;
        while !src.is_empty() {
            let dst = match self.cur_buf {
                Some(mut buf) => &mut buf.as_mut()[self.buf_idx..],
                None => break,
            };
            match bitcopy_nonoverlapping(dst, src) {
                BufferState::DstFull | BufferState::SameSize => {
                    written += dst.len();
                    src = &src[dst.len()..];
                    self.advance_cur();
                }
                BufferState::SrcEmpty => {
                    written += src.len();
                    self.buf_idx += src.len();
                    break;
                }
            }
        }
        written
    }

    fn advance_cur<'a>(&mut self) -> Option<&'a mut [MaybeUninit<T>]> {
        self.cur_buf.and_then(move |_| match self.buf {
            BufOwn::Single(_) => {
                self.cur_buf = None;
                self.buf_idx = 0;
                None
            }
        })
    }
}

impl<T: Pod> BufWriter<'_, T> {
    pub fn write(&mut self, src: &[T]) -> usize {
        unsafe { self.write_bitcopy(maybe_uninit_init_slice(src)) }
    }

    pub fn write_bytes(&mut self, _value: u8, _count: usize) -> usize {
        todo!()
    }
}

pub struct BufReader<'a, T> {
    buf: &'a BufOwn<T>,
    cur_buf: Option<&'a [MaybeUninit<T>]>,
    buf_idx: usize,
}

impl<T> BufReader<'_, T> {
    pub fn read_uninit(&mut self, mut dst: &mut [MaybeUninit<T>]) -> usize {
        let mut read = 0;
        while !dst.is_empty() {
            let src = match self.cur_buf {
                Some(cur_buf) => &cur_buf[self.buf_idx..],
                None => break,
            };
            match unsafe { bitcopy_nonoverlapping(dst, src) } {
                BufferState::SrcEmpty | BufferState::SameSize => {
                    read += src.len();
                    dst = &mut dst[src.len()..];
                    self.advance_cur();
                }
                BufferState::DstFull => {
                    read += dst.len();
                    self.buf_idx += dst.len();
                    break;
                }
            }
        }
        read
    }

    fn advance_cur<'a>(&mut self) -> Option<&'a [MaybeUninit<T>]> {
        self.cur_buf.and_then(move |_| match self.buf {
            BufOwn::Single(_) => {
                self.cur_buf = None;
                self.buf_idx = 0;
                None
            }
        })
    }
}

impl<T: Pod> BufReader<'_, T> {
    pub fn read(&mut self, dst: &mut [T]) -> usize {
        self.read_uninit(maybe_uninit_init_slice_mut(dst))
    }
}
