use core::cmp::min;
use core::fmt;
use core::marker::PhantomData;
use core::mem::size_of;
use core::ops::{Bound, Index, IndexMut, RangeBounds};

#[derive(Clone, Copy)]
pub struct StrideSlice<'a, T> {
    ptr: *const T,
    len: usize,
    stride: usize,
    array: PhantomData<&'a [T]>,
}

unsafe impl<T: Send> Send for StrideSlice<'_, T> {}
unsafe impl<T: Sync> Sync for StrideSlice<'_, T> {}

pub struct StrideSliceMut<'a, T> {
    ptr: *mut T,
    len: usize,
    stride: usize,
    array: PhantomData<&'a mut [T]>,
}
unsafe impl<T: Send> Send for StrideSliceMut<'_, T> {}
unsafe impl<T: Sync> Sync for StrideSliceMut<'_, T> {}

macro_rules! stride_slice_impl {
    ($ptr:ident, $Stride:ident) => {
        impl<'a, T> $Stride<'a, T> {
            pub fn subslice<R: RangeBounds<usize>>(&self, r: R) -> StrideSlice<'_, T> {
                unsafe {
                    let (start, _, len) = self.ptr_range(r);
                    StrideSlice {
                        ptr: start,
                        len: len,
                        stride: self.stride,
                        array: PhantomData,
                    }
                }
            }

            pub fn len(&self) -> usize {
                self.len
            }

            pub fn stride(&self) -> usize {
                self.stride
            }

            pub fn iter(&self) -> StrideSliceIter<'a, T> {
                StrideSliceIter {
                    cur: self.ptr,
                    end: unsafe { self.ptr_offset(self.len) },
                    stride: self.stride,
                    array: PhantomData,
                }
            }

            unsafe fn ptr_offset(&self, offset: usize) -> *$ptr T {
                let ptr = self.ptr.cast::<u8>();
                let ptr = ptr.add(offset * self.stride);
                ptr.cast()
            }

            unsafe fn ptr_range<R: RangeBounds<usize>>(&self, r: R) -> (*$ptr T, *$ptr T, usize) {
                let start;
                let end;
                match r.start_bound() {
                    Bound::Included(&idx) => start = min(idx, self.len),
                    Bound::Excluded(&idx) => start = min(idx + 1, self.len),
                    Bound::Unbounded => start = 0,
                };
                match r.end_bound() {
                    Bound::Included(&idx) => end = min(idx + 1, self.len),
                    Bound::Excluded(&idx) => end = min(idx, self.len),
                    Bound::Unbounded => end = self.len,
                };
                let len = end - start;
                (self.ptr_offset(start), self.ptr_offset(end), len)
            }
        }

        impl<T> Index<usize> for $Stride<'_, T> {
            type Output = T;
            fn index(&self, index: usize) -> &Self::Output {
                assert!(index < self.len, "Index [{}] out of range", index);
                unsafe { &*self.ptr_offset(index) }
            }
        }

        impl<'a, T> IntoIterator for $Stride<'a, T> {
            type Item = &'a T;
            type IntoIter = StrideSliceIter<'a, T>;
            fn into_iter(self) -> Self::IntoIter {
                self.iter()
            }
        }

        impl<'a, T> IntoIterator for &$Stride<'a, T> {
            type Item = &'a T;
            type IntoIter = StrideSliceIter<'a, T>;
            fn into_iter(self) -> Self::IntoIter {
                self.iter()
            }
        }

        impl<'a, T: fmt::Debug> fmt::Debug for $Stride<'a, T> {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                let mut l = f.debug_list();
                for i in self {
                    l.entry(i);
                }
                l.finish()
            }
        }
    };
    (const $Stride:ident) => { stride_slice_impl!(const, $Stride); };
    (mut $Stride:ident) => { stride_slice_impl!(mut, $Stride); };
}

stride_slice_impl!(const StrideSlice);
stride_slice_impl!(mut StrideSliceMut);

impl<'a, T> From<&'a [T]> for StrideSlice<'a, T> {
    fn from(s: &'a [T]) -> Self {
        Self {
            ptr: s.as_ptr(),
            len: s.len(),
            stride: size_of::<T>(),
            array: PhantomData,
        }
    }
}

impl<'a, T, const N: usize> From<&'a [T; N]> for StrideSlice<'a, T> {
    fn from(s: &'a [T; N]) -> Self {
        Self {
            ptr: s.as_ptr(),
            len: s.len(),
            stride: size_of::<T>(),
            array: PhantomData,
        }
    }
}

impl<'a, T> From<&'a mut [T]> for StrideSlice<'a, T> {
    fn from(s: &'a mut [T]) -> Self {
        Self {
            ptr: s.as_ptr(),
            len: s.len(),
            stride: size_of::<T>(),
            array: PhantomData,
        }
    }
}

impl<'a, T, const N: usize> From<&'a mut [T; N]> for StrideSlice<'a, T> {
    fn from(s: &'a mut [T; N]) -> Self {
        Self {
            ptr: s.as_ptr(),
            len: s.len(),
            stride: size_of::<T>(),
            array: PhantomData,
        }
    }
}

impl<'a, T> From<StrideSliceMut<'a, T>> for StrideSlice<'a, T> {
    fn from(s: StrideSliceMut<'a, T>) -> Self {
        Self {
            ptr: s.ptr,
            len: s.len,
            stride: s.stride,
            array: PhantomData,
        }
    }
}

impl<T> IndexMut<usize> for StrideSliceMut<'_, T> {
    fn index_mut(&mut self, index: usize) -> &mut T {
        assert!(index < self.len, "Index [{}] out of range", index);
        unsafe { &mut *self.ptr_offset(index) }
    }
}

impl<'a, T> StrideSliceMut<'a, T> {
    pub fn subslice_mut(&mut self, r: impl RangeBounds<usize>) -> StrideSliceMut<'_, T> {
        unsafe {
            let (start, _, len) = self.ptr_range(r);
            StrideSliceMut {
                ptr: start,
                len: len,
                stride: self.stride,
                array: PhantomData,
            }
        }
    }
    pub fn iter_mut(&mut self) -> StrideSliceIterMut<'a, T> {
        StrideSliceIterMut {
            cur: self.ptr,
            end: unsafe { self.ptr_offset(self.len) },
            stride: self.stride,
            array: PhantomData,
        }
    }
}

impl<'a, T> From<&'a mut [T]> for StrideSliceMut<'a, T> {
    fn from(s: &'a mut [T]) -> Self {
        Self {
            ptr: s.as_mut_ptr(),
            len: s.len(),
            stride: size_of::<T>(),
            array: PhantomData,
        }
    }
}

impl<'a, T, const N: usize> From<&'a mut [T; N]> for StrideSliceMut<'a, T> {
    fn from(s: &'a mut [T; N]) -> Self {
        Self {
            ptr: s.as_mut_ptr(),
            len: s.len(),
            stride: size_of::<T>(),
            array: PhantomData,
        }
    }
}

#[derive(Clone)]
pub struct StrideSliceIter<'a, T> {
    cur: *const T,
    end: *const T,
    stride: usize,
    array: PhantomData<&'a [T]>,
}

impl<'a, T> Iterator for StrideSliceIter<'a, T> {
    type Item = &'a T;
    fn next(&mut self) -> Option<Self::Item> {
        if self.cur < self.end {
            let ptr = self.cur;
            unsafe {
                self.cur = self.cur.cast::<u8>().offset(self.stride as _).cast();
                Some(&*ptr)
            }
        } else {
            None
        }
    }
}

pub struct StrideSliceIterMut<'a, T> {
    cur: *mut T,
    end: *mut T,
    stride: usize,
    array: PhantomData<&'a mut [T]>,
}

impl<'a, T> Iterator for StrideSliceIterMut<'a, T> {
    type Item = &'a mut T;
    fn next(&mut self) -> Option<Self::Item> {
        if self.cur < self.end {
            let ptr = self.cur;
            unsafe {
                self.cur = self.cur.cast::<u8>().offset(self.stride as _).cast();
                Some(&mut *ptr)
            }
        } else {
            None
        }
    }
}

/**
Creates a StrideSlice from
# Safety
Behavior is undefined if any of the following conditions are violated
- `ptr` mut be valid for reads
*/
pub unsafe fn from_raw_parts<'a, T>(
    ptr: *const T,
    len: usize,
    stride: usize,
) -> StrideSlice<'a, T> {
    StrideSlice {
        ptr,
        len,
        stride,
        array: PhantomData,
    }
}

pub unsafe fn from_raw_parts_mut<'a, T>(
    ptr: *mut T,
    len: usize,
    stride: usize,
) -> StrideSliceMut<'a, T> {
    StrideSliceMut {
        ptr,
        len,
        stride,
        array: PhantomData,
    }
}
