use core::hint::unreachable_unchecked;

pub trait CeilDiv {
    fn ceil_div(self, rhs: Self) -> Self;

    fn align_up(self, align: Self) -> Self;
}

macro_rules! ceil_div_impl {
    ($($ty:ty),* $(,)?) => {
        $(
            impl CeilDiv for $ty {
                #[inline]
                fn ceil_div(self, rhs: Self) -> Self {
                    (self + rhs - 1) / rhs
                }

                #[inline]
                fn align_up(self, align: Self) -> Self {
                    self.ceil_div(align) * align
                }
            }
        )*
    };
}

ceil_div_impl!(u8, i8, u16, i16, u32, i32, u64, i64, u128, i128, usize, isize,);

pub const fn log2u64(value: u64) -> u32 {
    debug_assert!(value != 0, "Cannot calculate log of 0");
    63 - value.leading_zeros()
}

pub const fn ceil_log2u64(value: u64) -> u32 {
    debug_assert!(value != 0, "Cannot calculate log of 0");
    match value {
        0 => unsafe { unreachable_unchecked() },
        1 => 0,
        n => log2u64(next_pow2u64(n)),
    }
}

pub const fn next_pow2u64(mut value: u64) -> u64 {
    value -= 1;
    value |= value >> 1;
    value |= value >> 2;
    value |= value >> 4;
    value |= value >> 8;
    value |= value >> 16;
    value |= value >> 32;
    value + 1
}
