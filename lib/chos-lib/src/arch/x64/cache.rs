use core::fmt;
use core::hash::Hash;
use core::ops::{Deref, DerefMut};

macro_rules! forward_fmt {
    ($name:ident => $($fmt:ident),* $(,)?) => {
        $(
            impl<T: fmt::$fmt> fmt::$fmt for $name<T> {
                #[inline]
                fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                    fmt::$fmt::fmt(&self.value, f)
                }
            }
        )*
    };
}

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

    #[inline]
    pub fn into_inner(self) -> T {
        self.value
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

forward_fmt!(CacheAligned => Debug, Binary, LowerHex, UpperHex, LowerExp, UpperExp, Octal, Display, Pointer);
