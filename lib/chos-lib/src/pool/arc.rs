#[cfg(feature = "alloc")]
use alloc::boxed::Box;
#[cfg(feature = "alloc")]
use core::alloc::Allocator;
use core::alloc::{AllocError, Layout};
use core::convert::TryFrom;
use core::fmt;
use core::marker::{PhantomData, Unpin, Unsize};
use core::ops::{CoerceUnsized, Deref};
use core::ptr::{drop_in_place, NonNull};
use core::sync::atomic::{fence, AtomicUsize, Ordering};

use intrusive_collections::{PointerOps, TryExclusivePointerOps};

use super::{handle_alloc_error, ConstPool, Pool, PoolBox};
use crate::init::ConstInit;
use crate::intrusive::DefaultPointerOps;

mod private {
    pub trait Sealed {}
}

pub trait Count: private::Sealed {
    fn acquire_strong(&self);
    unsafe fn release_strong<T: ?Sized>(&self, ptr: NonNull<T>, pool: &impl Pool<T>);
    fn strong_count(&self) -> usize;
}

pub trait WeakCount: Count {
    unsafe fn init_count(this: *const Self, strong: usize, weak: usize);
    unsafe fn upgrade(this: *const Self) -> bool;
    unsafe fn acquire_weak(this: *const Self);
    unsafe fn release_weak<T: ?Sized>(this: *const Self, ptr: NonNull<T>, pool: &impl Pool<T>);

    unsafe fn strong_count(this: *const Self) -> usize;
    unsafe fn weak_count(this: *const Self) -> usize;
}

pub struct IArcCount {
    strong: AtomicUsize,
}

impl IArcCount {
    pub const fn new() -> Self {
        Self {
            strong: AtomicUsize::new(0),
        }
    }
}

impl ConstInit for IArcCount {
    #[allow(clippy::declare_interior_mutable_const)]
    const INIT: Self = Self::new();
}

impl fmt::Debug for IArcCount {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("IArcCount").finish()
    }
}

impl private::Sealed for IArcCount {}
impl Count for IArcCount {
    fn acquire_strong(&self) {
        self.strong.fetch_add(1, Ordering::Acquire);
    }
    unsafe fn release_strong<T: ?Sized>(&self, ptr: NonNull<T>, pool: &impl Pool<T>) {
        if self.strong.fetch_sub(1, Ordering::Release) == 1 {
            drop_in_place(ptr.as_ptr());
            pool.deallocate(ptr, Layout::for_value_raw(ptr.as_ptr()));
        }
    }
    fn strong_count(&self) -> usize {
        self.strong.load(Ordering::Relaxed)
    }
}

pub struct IArcCountWeak {
    strong: AtomicUsize,
    weak: AtomicUsize,
}

impl IArcCountWeak {
    pub const fn new() -> Self {
        Self {
            strong: AtomicUsize::new(0),
            weak: AtomicUsize::new(0),
        }
    }
}

impl ConstInit for IArcCountWeak {
    #[allow(clippy::declare_interior_mutable_const)]
    const INIT: Self = Self::new();
}

impl fmt::Debug for IArcCountWeak {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("IArcCountWeak").finish()
    }
}

impl private::Sealed for IArcCountWeak {}
impl Count for IArcCountWeak {
    fn acquire_strong(&self) {
        if self.strong.fetch_add(1, Ordering::Acquire) == 0 {
            unsafe { Self::acquire_weak(self) }
        }
    }

    unsafe fn release_strong<T: ?Sized>(&self, ptr: NonNull<T>, pool: &impl Pool<T>) {
        if self.strong.fetch_sub(1, Ordering::Release) == 1 {
            drop_in_place(ptr.as_ptr());
            Self::release_weak(self, ptr, pool)
        }
    }

    fn strong_count(&self) -> usize {
        self.strong.load(Ordering::Relaxed)
    }
}

impl WeakCount for IArcCountWeak {
    unsafe fn init_count(this: *const Self, strong: usize, weak: usize) {
        (*this).strong.store(strong, Ordering::Relaxed);
        (*this).weak.store(weak, Ordering::Relaxed);
    }

    unsafe fn upgrade(this: *const Self) -> bool {
        let strong = &(*this).strong;
        loop {
            let str_count = strong.load(Ordering::Relaxed);
            if str_count == 0 {
                return false;
            }
            if strong
                .compare_exchange_weak(
                    str_count,
                    str_count + 1,
                    Ordering::Acquire,
                    Ordering::Relaxed,
                )
                .is_ok()
            {
                return true;
            }
        }
    }

    unsafe fn acquire_weak(this: *const Self) {
        let weak = &(*this).weak;
        weak.fetch_add(1, Ordering::Relaxed);
    }

    unsafe fn release_weak<T: ?Sized>(this: *const Self, ptr: NonNull<T>, pool: &impl Pool<T>) {
        let weak = &(*this).weak;
        if weak.fetch_sub(1, Ordering::Relaxed) == 1 {
            pool.deallocate(ptr, Layout::for_value_raw(ptr.as_ptr()));
        }
    }

    unsafe fn strong_count(this: *const Self) -> usize {
        let strong = &(*this).strong;
        strong.load(Ordering::Relaxed)
    }

    unsafe fn weak_count(this: *const Self) -> usize {
        let weak = &(*this).weak;
        weak.load(Ordering::Relaxed)
    }
}

pub trait IArcAdapter {
    type Count: Count = IArcCount;
    unsafe fn count(this: *const Self) -> *const Self::Count;
}

pub struct IArc<T: IArcAdapter + ?Sized, P: Pool<T>> {
    ptr: NonNull<T>,
    alloc: P,
    value: PhantomData<T>,
}

impl<T: IArcAdapter + ?Sized, P: Pool<T>> IArc<T, P> {
    pub fn try_new_in(value: T, alloc: P) -> Result<Self, AllocError>
    where
        T: Sized,
    {
        Self::count(&value).acquire_strong();
        let ptr = unsafe { alloc.allocate()? };
        unsafe { core::ptr::write(ptr.as_ptr(), value) };
        Ok(Self {
            ptr,
            alloc,
            value: PhantomData,
        })
    }

    pub fn new_in(value: T, alloc: P) -> Self
    where
        T: Sized,
    {
        let r = Self::try_new_in(value, alloc);
        r.unwrap_or_else(|_| handle_alloc_error(Layout::new::<T>()))
    }

    pub fn into_raw_with_allocator(this: Self) -> (*const T, P) {
        let ptr = this.ptr.as_ptr();
        let alloc = unsafe { core::ptr::read(&this.alloc) };
        core::mem::forget(this);
        (ptr, alloc)
    }

    pub unsafe fn from_raw_in(ptr: *const T, alloc: P) -> Self {
        Self {
            ptr: NonNull::new_unchecked(ptr as _),
            alloc,
            value: PhantomData,
        }
    }

    pub fn get_ptr(&self) -> *const T {
        self.ptr.as_ptr()
    }

    pub fn get_ref(&self) -> &T {
        unsafe { self.ptr.as_ref() }
    }

    pub fn get_mut(this: &mut Self) -> Option<&mut T> {
        this.is_unique()
            .then(move || unsafe { Self::get_mut_unchecked(this) })
    }

    pub unsafe fn get_mut_unchecked(this: &mut Self) -> &mut T {
        this.ptr.as_mut()
    }

    pub fn strong_count(&self) -> usize {
        Self::count(self).strong_count()
    }

    pub fn is_unique(&self) -> bool {
        self.strong_count() == 1
    }

    #[cfg(feature = "alloc")]
    pub fn from_box(b: Box<T, P>) -> Self
    where
        P: Allocator,
    {
        Self::count(&b).acquire_strong();
        let (ptr, alloc) = Box::into_raw_with_allocator(b);
        Self {
            ptr: unsafe { NonNull::new_unchecked(ptr) },
            alloc,
            value: PhantomData,
        }
    }

    pub fn from_pool_box(b: PoolBox<T, P>) -> Self {
        let (ptr, alloc) = PoolBox::leak_with_allocator(b);
        Self::count(ptr).acquire_strong();
        Self {
            ptr: ptr.into(),
            alloc,
            value: PhantomData,
        }
    }

    #[cfg(feature = "alloc")]
    pub fn into_box(this: Self) -> Result<Box<T, P>, Self>
    where
        P: Allocator,
    {
        if this.is_unique() {
            unsafe {
                Self::count(&this).release_strong(this.ptr, &this.alloc);
            }
            let (ptr, alloc) = Self::into_raw_with_allocator(this);
            unsafe { Ok(Box::from_raw_in(ptr as _, alloc)) }
        } else {
            Err(this)
        }
    }

    pub fn into_pool_box(this: Self) -> Result<PoolBox<T, P>, Self> {
        if this.is_unique() {
            unsafe {
                Self::count(&this).release_strong(this.ptr, &this.alloc);
            }
            let (ptr, alloc) = Self::into_raw_with_allocator(this);
            unsafe { Ok(PoolBox::from_raw_in(ptr as _, alloc)) }
        } else {
            Err(this)
        }
    }

    fn count(value: &T) -> &T::Count {
        unsafe { &*IArcAdapter::count(value) }
    }
}

impl<T: IArcAdapter + ?Sized, P: ConstPool<T>> IArc<T, P> {
    pub fn new(value: T) -> Self
    where
        T: Sized,
    {
        Self::try_new_in(value, P::INIT)
            .unwrap_or_else(|_| super::handle_alloc_error(Layout::new::<T>()))
    }

    pub fn try_new(value: T) -> Result<Self, AllocError>
    where
        T: Sized,
    {
        Self::try_new_in(value, P::INIT)
    }

    pub fn into_raw(this: Self) -> *const T {
        let ptr = this.ptr.as_ptr();
        core::mem::forget(this);
        ptr
    }

    pub unsafe fn from_raw(raw: *const T) -> Self {
        Self {
            ptr: NonNull::new_unchecked(raw as *mut T),
            alloc: P::INIT,
            value: PhantomData,
        }
    }
}
impl<T: IArcAdapter<Count: WeakCount> + ?Sized, P: ConstPool<T>> IArc<T, P> {
    pub fn try_new_cyclic(f: impl FnOnce(&IWeak<T, P>) -> T) -> Result<Self, AllocError>
    where
        T: Sized,
    {
        let alloc = P::INIT;
        unsafe {
            let ptr = alloc.allocate()?;
            let ptr_count = IArcAdapter::count(ptr.as_ptr());
            WeakCount::init_count(ptr_count, 0, 1);
            let weak = IWeak {
                ptr,
                alloc,
                value: PhantomData,
            };
            let value = f(&weak);
            let value_count = IArcAdapter::count(&value);
            assert_eq!(WeakCount::strong_count(ptr_count), 0);
            WeakCount::init_count(value_count, 0, WeakCount::weak_count(ptr_count));
            core::ptr::write(ptr.as_ptr(), value);
            Count::acquire_strong(&*ptr_count);
            Ok(IArc {
                ptr,
                alloc,
                value: PhantomData,
            })
        }
    }

    pub fn new_cyclic(f: impl FnOnce(&IWeak<T, P>) -> T) -> IArc<T, P>
    where
        T: Sized,
    {
        Self::try_new_cyclic(f).expect("Allocation error")
    }

    pub fn weak_count(&self) -> usize {
        unsafe { WeakCount::weak_count(IArcAdapter::count(&**self)) }
    }

    pub fn downgrade(this: &Self) -> IWeak<T, P> {
        unsafe { WeakCount::acquire_weak(IArcAdapter::count(&**this)) };
        IWeak {
            ptr: this.ptr,
            alloc: this.alloc,
            value: PhantomData,
        }
    }
}

impl<T: IArcAdapter + ?Sized, P: Pool<T>> Drop for IArc<T, P> {
    fn drop(&mut self) {
        unsafe {
            Self::count(&self).release_strong(self.ptr, &self.alloc);
        }
        fence(Ordering::Acquire);
    }
}

impl<T: IArcAdapter + ?Sized, P: Pool<T> + Clone> Clone for IArc<T, P> {
    fn clone(&self) -> Self {
        Self::count(self).acquire_strong();
        Self {
            ptr: self.ptr,
            alloc: P::clone(&self.alloc),
            value: PhantomData,
        }
    }
}

impl<T: IArcAdapter + ?Sized, P: Pool<T>> Deref for IArc<T, P> {
    type Target = T;
    fn deref(&self) -> &Self::Target {
        self.get_ref()
    }
}

impl<T: IArcAdapter + PartialEq + ?Sized, P: Pool<T>> PartialEq for IArc<T, P> {
    fn eq(&self, other: &Self) -> bool {
        <T as PartialEq>::eq(self, other)
    }
}
impl<T: IArcAdapter + PartialEq + ?Sized, P: Pool<T>> PartialEq<T> for IArc<T, P> {
    fn eq(&self, other: &T) -> bool {
        <T as PartialEq>::eq(self, other)
    }
}
impl<T: IArcAdapter + Eq + ?Sized, P: Pool<T>> Eq for IArc<T, P> {}

impl<T: IArcAdapter + PartialOrd + ?Sized, P: Pool<T>> PartialOrd for IArc<T, P> {
    fn partial_cmp(&self, other: &Self) -> Option<core::cmp::Ordering> {
        <T as PartialOrd>::partial_cmp(self, other)
    }
}
impl<T: IArcAdapter + PartialOrd + ?Sized, P: Pool<T>> PartialOrd<T> for IArc<T, P> {
    fn partial_cmp(&self, other: &T) -> Option<core::cmp::Ordering> {
        <T as PartialOrd>::partial_cmp(self, other)
    }
}
impl<T: IArcAdapter + Ord + ?Sized, P: Pool<T>> Ord for IArc<T, P> {
    fn cmp(&self, other: &Self) -> core::cmp::Ordering {
        <T as Ord>::cmp(self, other)
    }
}

impl<T: IArcAdapter + ?Sized, P: Unpin + Pool<T>> Unpin for IArc<T, P> {}
unsafe impl<T: IArcAdapter + Sync + Send + ?Sized, P: Pool<T> + Sync> Sync for IArc<T, P> {}
unsafe impl<T: IArcAdapter + Sync + Send + ?Sized, P: Pool<T> + Send> Send for IArc<T, P> {}

impl<T, U, P> CoerceUnsized<IArc<U, P>> for IArc<T, P>
where
    T: IArcAdapter + Unsize<U> + ?Sized,
    U: IArcAdapter + ?Sized,
    P: Pool<T> + Pool<U>,
{
}

macro_rules! fmt {
    ($($fmt:ident),* $(,)?) => {
        $(
            impl<T: IArcAdapter + fmt::$fmt + ?Sized, P: Pool<T>> fmt::$fmt for IArc<T, P> {
                fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
                    fmt::$fmt::fmt(self.get_ref(), f)
                }
            }
        )*
    };
}

fmt!(Debug, Display, Binary, Octal, LowerHex, UpperHex, LowerExp, UpperExp,);

impl<T: IArcAdapter + ?Sized, P: Pool<T>> fmt::Pointer for IArc<T, P> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        fmt::Pointer::fmt(&self.ptr, f)
    }
}

unsafe impl<T: IArcAdapter + ?Sized, P: ConstPool<T>> PointerOps for DefaultPointerOps<IArc<T, P>> {
    type Value = T;
    type Pointer = IArc<T, P>;

    unsafe fn from_raw(&self, value: *const Self::Value) -> Self::Pointer {
        IArc::from_raw(value as *mut Self::Value)
    }

    fn into_raw(&self, ptr: Self::Pointer) -> *const Self::Value {
        IArc::into_raw(ptr)
    }
}

unsafe impl<T: IArcAdapter + ?Sized, P: ConstPool<T>> TryExclusivePointerOps
    for DefaultPointerOps<IArc<T, P>>
{
    unsafe fn try_get_mut(&self, value: *const Self::Value) -> Option<*mut Self::Value> {
        let mut arc = IArc::<T, P>::from_raw(value);
        let res = IArc::get_mut(&mut arc).map(|res| res as *mut T);
        drop(IArc::into_raw(arc));
        res
    }
}

impl<T: IArcAdapter, P: ConstPool<T>> From<T> for IArc<T, P> {
    fn from(v: T) -> Self {
        IArc::new(v)
    }
}

impl<T: IArcAdapter + ?Sized, P: Pool<T>> From<PoolBox<T, P>> for IArc<T, P> {
    fn from(b: PoolBox<T, P>) -> Self {
        Self::from_pool_box(b)
    }
}

impl<T: IArcAdapter + ?Sized, P: Pool<T>> TryFrom<IArc<T, P>> for PoolBox<T, P> {
    type Error = IArc<T, P>;
    fn try_from(value: IArc<T, P>) -> Result<Self, Self::Error> {
        IArc::into_pool_box(value)
    }
}

pub struct IWeak<T: IArcAdapter<Count: WeakCount> + ?Sized, P: ConstPool<T>> {
    ptr: NonNull<T>,
    alloc: P,
    value: PhantomData<T>,
}
unsafe impl<T: IArcAdapter<Count: WeakCount> + Sync + Send + ?Sized, P: ConstPool<T> + Sync> Sync
    for IWeak<T, P>
{
}
unsafe impl<T: IArcAdapter<Count: WeakCount> + Sync + Send + ?Sized, P: ConstPool<T> + Send> Send
    for IWeak<T, P>
{
}

impl<T: IArcAdapter<Count: WeakCount> + ?Sized, P: ConstPool<T>> IWeak<T, P> {
    pub fn upgrade(&self) -> Option<IArc<T, P>> {
        unsafe { WeakCount::upgrade(IArcAdapter::count(self.ptr.as_ptr())) }.then(|| IArc {
            ptr: self.ptr,
            alloc: self.alloc,
            value: PhantomData,
        })
    }

    pub fn strong_count(&self) -> usize {
        unsafe { WeakCount::strong_count(IArcAdapter::count(self.ptr.as_ptr())) }
    }

    pub fn weak_count(&self) -> usize {
        unsafe { WeakCount::weak_count(IArcAdapter::count(self.ptr.as_ptr())) }
    }
}

impl<T: IArcAdapter<Count: WeakCount> + ?Sized, P: ConstPool<T>> Drop for IWeak<T, P> {
    fn drop(&mut self) {
        unsafe {
            WeakCount::release_weak(IArcAdapter::count(self.ptr.as_ptr()), self.ptr, &self.alloc);
        }
    }
}

impl<T: IArcAdapter<Count: WeakCount> + ?Sized, P: ConstPool<T>> Clone for IWeak<T, P> {
    fn clone(&self) -> Self {
        unsafe { WeakCount::acquire_weak(IArcAdapter::count(self.ptr.as_ptr())) };
        Self {
            ptr: self.ptr,
            alloc: self.alloc,
            value: PhantomData,
        }
    }
}

impl<T: IArcAdapter<Count: WeakCount> + fmt::Debug + ?Sized, P: ConstPool<T>> fmt::Debug
    for IWeak<T, P>
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        struct Dropped;
        impl fmt::Debug for Dropped {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                f.write_str("<dropped>")
            }
        }
        match Self::upgrade(self) {
            Some(arc) => fmt::Debug::fmt(&arc, f),
            None => fmt::Debug::fmt(&Dropped, f),
        }
    }
}

impl<T: IArcAdapter<Count: WeakCount> + ?Sized, P: ConstPool<T>> fmt::Pointer for IWeak<T, P> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Pointer::fmt(&self.ptr, f)
    }
}

pub macro iarc_adapter($name:path : $field:ident) {
    impl $crate::pool::arc::IArcAdapter for $name {
        unsafe fn count(this: *const Self) -> *const Self::Count {
            &(*this).$field
        }
    }
}

pub macro iarc_adapter_weak($name:path : $field:ident) {
    impl $crate::pool::arc::IArcAdapter for $name {
        type Count = $crate::pool::arc::IArcCountWeak;
        unsafe fn count(this: *const Self) -> *const Self::Count {
            &(*this).$field
        }
    }
}
