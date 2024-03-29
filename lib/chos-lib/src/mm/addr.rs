use core::ptr::{from_raw_parts, from_raw_parts_mut, Pointee};
use core::{fmt, ops};

use super::{FrameSize, PFrame, VFrame};
use crate::arch::mm::{is_canonical, make_canonical, DefaultFrameSize, PAddrRepr, VAddrRepr};
use crate::int::align_upu64;

#[repr(transparent)]
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
pub struct PAddr(u64);

impl PAddr {
    pub const fn null() -> Self {
        Self(0)
    }

    pub const fn new(v: PAddrRepr) -> Self {
        Self(v)
    }

    pub const fn from_u64(v: u64) -> Self {
        Self::new(v as PAddrRepr)
    }

    pub const fn from_usize(v: usize) -> Self {
        Self::new(v as PAddrRepr)
    }

    pub const fn as_repr(self) -> PAddrRepr {
        self.0
    }

    pub const fn as_u64(self) -> u64 {
        self.0 as u64
    }

    pub const fn as_usize(self) -> usize {
        self.0 as usize
    }

    pub const fn is_page_aligned(self) -> bool {
        (self.0 & !DefaultFrameSize::PAGE_MASK) == 0
    }

    pub const fn align_page_up(self) -> Self {
        Self::new(align_upu64(self.0, DefaultFrameSize::PAGE_SIZE))
    }

    pub const fn align_page_down(self) -> Self {
        Self::new(self.0 / DefaultFrameSize::PAGE_SIZE * DefaultFrameSize::PAGE_SIZE)
    }

    pub const fn page(self) -> PAddrRepr {
        self.0 >> DefaultFrameSize::PAGE_SHIFT
    }

    pub const fn add_u64(self, rhs: u64) -> Self {
        Self::new(self.0 + rhs)
    }

    pub const fn add_paddr(self, rhs: PAddr) -> Self {
        Self::new(self.0 + rhs.0)
    }

    pub const fn sub_u64(self, rhs: u64) -> Self {
        Self::new(self.0 - rhs)
    }

    pub const fn sub_paddr(self, rhs: PAddr) -> Self {
        Self::new(self.0 - rhs.0)
    }

    pub const fn frame_offset<S: FrameSize>(&self) -> (PFrame<S>, u64) {
        let page = self.0 & S::PAGE_MASK;
        let off = self.0 & !S::PAGE_MASK;
        (unsafe { PFrame::new_unchecked(PAddr::new(page)) }, off)
    }

    pub const fn identity(self) -> VAddr {
        VAddr::null().add_paddr(self)
    }
}

impl ops::Add<u64> for PAddr {
    type Output = PAddr;
    fn add(self, rhs: u64) -> Self::Output {
        PAddr::new(self.0 + rhs)
    }
}

impl ops::AddAssign<u64> for PAddr {
    fn add_assign(&mut self, rhs: u64) {
        *self = *self + rhs;
    }
}

impl ops::Add<PAddr> for u64 {
    type Output = PAddr;
    fn add(self, rhs: PAddr) -> Self::Output {
        PAddr::new(self + rhs.0)
    }
}

impl ops::Add for PAddr {
    type Output = PAddr;
    fn add(self, rhs: Self) -> Self::Output {
        PAddr::new(self.0 + rhs.0)
    }
}

impl ops::AddAssign for PAddr {
    fn add_assign(&mut self, rhs: PAddr) {
        *self = *self + rhs;
    }
}

impl ops::Sub<u64> for PAddr {
    type Output = PAddr;
    fn sub(self, rhs: u64) -> Self::Output {
        PAddr::new(self.0 - rhs)
    }
}

impl ops::SubAssign<u64> for PAddr {
    fn sub_assign(&mut self, rhs: u64) {
        *self = *self - rhs;
    }
}

impl ops::Sub<PAddr> for u64 {
    type Output = PAddr;
    fn sub(self, rhs: PAddr) -> Self::Output {
        PAddr::new(self - rhs.0)
    }
}

impl ops::Sub for PAddr {
    type Output = PAddr;
    fn sub(self, rhs: Self) -> Self::Output {
        PAddr::new(self.0 - rhs.0)
    }
}

impl ops::SubAssign for PAddr {
    fn sub_assign(&mut self, rhs: PAddr) {
        *self = *self - rhs;
    }
}

crate::forward_fmt!(impl LowerHex, UpperHex for PAddr => u64 : |this: &Self| (*this).as_u64());
impl fmt::Debug for PAddr {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "PAddr(0x{:x})", self.0)
    }
}
impl fmt::Display for PAddr {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::LowerHex::fmt(self, f)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct NonCanonicalError;

#[repr(transparent)]
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
pub struct VAddr(u64);

impl VAddr {
    pub const fn null() -> Self {
        Self(0)
    }

    pub const fn try_new(addr: u64) -> Result<Self, NonCanonicalError> {
        if is_canonical(addr) {
            Ok(Self(addr))
        } else {
            Err(NonCanonicalError)
        }
    }

    pub const fn try_from_usize(v: usize) -> Result<Self, NonCanonicalError> {
        Self::try_new(v as u64)
    }

    pub const fn new(addr: u64) -> Self {
        match Self::try_new(addr) {
            Ok(vaddr) => vaddr,
            Err(_) => panic!("Non canonical address"),
        }
    }

    pub const fn from_usize(v: usize) -> Self {
        Self::new(v as u64)
    }

    pub const unsafe fn new_unchecked(addr: u64) -> Self {
        Self(addr)
    }

    pub const unsafe fn from_usize_unchecked(v: usize) -> Self {
        Self::new_unchecked(v as u64)
    }

    pub const fn make_canonical(addr: VAddrRepr) -> Self {
        unsafe { VAddr::new_unchecked(make_canonical(addr)) }
    }

    pub const fn as_u64(self) -> u64 {
        self.0
    }

    pub const fn as_usize(self) -> usize {
        self.0 as usize
    }

    pub const fn is_page_aligned(self) -> bool {
        (self.0 & !DefaultFrameSize::PAGE_MASK) == 0
    }

    pub const fn align_page_up(self) -> Self {
        Self::new(align_upu64(self.0, DefaultFrameSize::PAGE_SIZE))
    }

    pub const fn align_page_down(self) -> Self {
        Self::new(self.0 / DefaultFrameSize::PAGE_SIZE * DefaultFrameSize::PAGE_SIZE)
    }

    pub const fn page(self) -> u64 {
        self.0 >> DefaultFrameSize::PAGE_SHIFT
    }

    pub const fn frame_offset<S: FrameSize>(&self) -> (VFrame<S>, u64) {
        let page = self.0 & S::PAGE_MASK;
        let off = self.0 & !S::PAGE_MASK;
        (
            unsafe { VFrame::new_unchecked(VAddr::new_unchecked(page)) },
            off,
        )
    }

    pub const fn as_ptr<T>(self) -> *const T {
        self.0 as _
    }

    pub const fn as_mut_ptr<T>(self) -> *mut T {
        self.0 as _
    }

    pub const fn from_raw_parts_ptr<T: Pointee + ?Sized>(self, metadata: T::Metadata) -> *const T {
        from_raw_parts(self.as_ptr(), metadata)
    }

    pub const fn from_raw_parts_mut_ptr<T: Pointee + ?Sized>(
        self,
        metadata: T::Metadata,
    ) -> *mut T {
        from_raw_parts_mut(self.as_mut_ptr(), metadata)
    }

    pub const unsafe fn as_ref<'a, T>(self) -> &'a T {
        &*self.as_ptr()
    }

    pub const unsafe fn as_mut<'a, T>(self) -> &'a mut T {
        &mut *self.as_mut_ptr()
    }

    pub const unsafe fn from_raw_parts<'a, T: Pointee + ?Sized>(
        self,
        metadata: T::Metadata,
    ) -> &'a T {
        &*self.from_raw_parts_ptr(metadata)
    }

    pub const unsafe fn from_raw_parts_mut<'a, T: Pointee + ?Sized>(
        self,
        metadata: T::Metadata,
    ) -> &'a mut T {
        &mut *self.from_raw_parts_mut_ptr(metadata)
    }

    pub const fn try_add_u64(self, other: u64) -> Result<Self, NonCanonicalError> {
        Self::try_new(self.0 + other)
    }

    pub const fn add_u64(self, other: u64) -> Self {
        Self::new(self.0 + other)
    }

    pub const fn try_add_paddr(self, other: PAddr) -> Result<Self, NonCanonicalError> {
        Self::try_new(self.0 + other.0)
    }

    pub const fn add_paddr(self, other: PAddr) -> Self {
        Self::new(self.0 + other.0)
    }

    pub const fn try_add_vaddr(self, other: VAddr) -> Result<Self, NonCanonicalError> {
        Self::try_new(self.0 + other.0)
    }

    pub const fn add_vaddr(self, other: VAddr) -> Self {
        Self::new(self.0 + other.0)
    }

    pub const fn try_sub_u64(self, other: u64) -> Result<Self, NonCanonicalError> {
        Self::try_new(self.0 - other)
    }

    pub const fn sub_u64(self, other: u64) -> Self {
        Self::new(self.0 - other)
    }

    pub const fn try_sub_paddr(self, other: PAddr) -> Result<Self, NonCanonicalError> {
        Self::try_new(self.0 - other.0)
    }

    pub const fn sub_paddr(self, other: PAddr) -> Self {
        Self::new(self.0 - other.0)
    }

    pub const fn try_sub_vaddr(self, other: VAddr) -> Result<Self, NonCanonicalError> {
        Self::try_new(self.0 - other.0)
    }

    pub const fn sub_vaddr(self, other: VAddr) -> Self {
        Self::new(self.0 - other.0)
    }

    pub const fn split(self) -> (u16, u16, u16, u16, u64) {
        let addr = self.0;
        let off = addr & 0xfff;
        let p1 = ((addr & (0x1ff << 12)) >> 12) as u16;
        let p2 = ((addr & (0x1ff << 21)) >> 21) as u16;
        let p3 = ((addr & (0x1ff << 30)) >> 30) as u16;
        let p4 = ((addr & (0x1ff << 39)) >> 39) as u16;
        (p4, p3, p2, p1, off)
    }
}

crate::forward_fmt!(impl Pointer for VAddr => *const () : |this: &Self| this.as_ptr::<()>());
crate::forward_fmt!(impl LowerHex, UpperHex for VAddr => u64 : |this: &Self| this.as_u64());
impl fmt::Debug for VAddr {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "VAddr(0x{:x})", self.0)
    }
}
impl fmt::Display for VAddr {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::LowerHex::fmt(self, f)
    }
}

impl ops::Add<u64> for VAddr {
    type Output = VAddr;
    fn add(self, rhs: u64) -> Self::Output {
        VAddr::new(self.0 + rhs)
    }
}

impl ops::Add<VAddr> for u64 {
    type Output = VAddr;
    fn add(self, rhs: VAddr) -> Self::Output {
        VAddr::new(self + rhs.0)
    }
}

impl ops::Add<PAddr> for VAddr {
    type Output = VAddr;
    fn add(self, rhs: PAddr) -> Self::Output {
        VAddr::new(self.0 + rhs.0)
    }
}

impl ops::Add<VAddr> for PAddr {
    type Output = VAddr;
    fn add(self, rhs: VAddr) -> Self::Output {
        VAddr::new(self.0 + rhs.0)
    }
}

impl ops::Add for VAddr {
    type Output = VAddr;
    fn add(self, rhs: Self) -> Self::Output {
        VAddr::new(self.0 + rhs.0)
    }
}

impl ops::Sub<u64> for VAddr {
    type Output = VAddr;
    fn sub(self, rhs: u64) -> Self::Output {
        VAddr::new(self.0 - rhs)
    }
}

impl ops::Sub<VAddr> for u64 {
    type Output = VAddr;
    fn sub(self, rhs: VAddr) -> Self::Output {
        VAddr::new(self - rhs.0)
    }
}

impl ops::Sub<PAddr> for VAddr {
    type Output = VAddr;
    fn sub(self, rhs: PAddr) -> Self::Output {
        VAddr::new(self.0 - rhs.0)
    }
}

impl ops::Sub<VAddr> for PAddr {
    type Output = VAddr;
    fn sub(self, rhs: VAddr) -> Self::Output {
        VAddr::new(self.0 - rhs.0)
    }
}

impl ops::Sub for VAddr {
    type Output = VAddr;
    fn sub(self, rhs: Self) -> Self::Output {
        VAddr::new(self.0 - rhs.0)
    }
}

impl<S: FrameSize> ops::Add for PFrame<S> {
    type Output = Self;
    fn add(self, rhs: Self) -> Self {
        unsafe { Self::new_unchecked(self.addr() + rhs.addr()) }
    }
}

impl<S: FrameSize> ops::Add<VFrame<S>> for PFrame<S> {
    type Output = VFrame<S>;
    fn add(self, rhs: VFrame<S>) -> VFrame<S> {
        unsafe { VFrame::new_unchecked(self.addr() + rhs.addr()) }
    }
}

impl<S: FrameSize> ops::Add<PFrame<S>> for VFrame<S> {
    type Output = VFrame<S>;
    fn add(self, rhs: PFrame<S>) -> VFrame<S> {
        unsafe { VFrame::new_unchecked(self.addr() + rhs.addr()) }
    }
}

impl<T: ?Sized> From<&T> for VAddr {
    fn from(v: &T) -> Self {
        unsafe { Self::new_unchecked(v as *const T as *const () as u64) }
    }
}

impl<T: ?Sized> From<&mut T> for VAddr {
    fn from(v: &mut T) -> Self {
        unsafe { Self::new_unchecked(v as *mut T as *mut () as u64) }
    }
}
