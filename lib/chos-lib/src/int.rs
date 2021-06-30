
pub trait CeilDiv {
    fn ceil_div(self, rhs: Self) -> Self;
}

macro_rules! ceil_div_impl {
    ($($ty:ty),* $(,)?) => {
        $(
            impl CeilDiv for $ty {
                #[inline]
                fn ceil_div(self, rhs: Self) -> Self {
                    (self + rhs - 1) / rhs
                }
            }
        )*
    };
}

ceil_div_impl!(
    u8, i8,
    u16, i16,
    u32, i32,
    u64, i64,
    u128, i128,
    usize, isize,
);
