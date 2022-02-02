use core::fmt;
use core::ops::{Deref, DerefMut};

pub const CACHE_LINE_SIZE: usize = 64;

#[repr(align(64))]
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
pub struct CacheAligned<T> {
    value: T,
}

impl<T> CacheAligned<T> {
    #[inline]
    pub const fn new(value: T) -> Self {
        Self { value }
    }
}

impl<T> Deref for CacheAligned<T> {
    type Target = T;
    #[inline]
    fn deref(&self) -> &Self::Target {
        &self.value
    }
}

impl<T> DerefMut for CacheAligned<T> {
    #[inline]
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.value
    }
}

macro_rules! forward_fmt {
    ($($fmt:ident),* $(,)?) => {
        $(
            impl<T: fmt::$fmt> fmt::$fmt for CacheAligned<T> {
                #[inline]
                fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                    fmt::$fmt::fmt(&self.value, f)
                }
            }
        )*
    };
}

forward_fmt!(Debug, Binary, LowerHex, UpperHex, LowerExp, UpperExp, Octal, Display, Pointer);
