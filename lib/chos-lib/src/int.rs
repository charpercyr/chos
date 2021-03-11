macro_rules! ceil_div_impl {
    ($name:ident, $ty:ty) => {
        pub const fn $name(a: $ty, b: $ty) -> $ty {
            (a + b - 1) / b
        }
    };
}

ceil_div_impl!(ceil_div_u8, u8);
ceil_div_impl!(ceil_div_i8, i8);
ceil_div_impl!(ceil_div_u16, u16);
ceil_div_impl!(ceil_div_i16, i16);
ceil_div_impl!(ceil_div_u32, u32);
ceil_div_impl!(ceil_div_i32, i32);
ceil_div_impl!(ceil_div_u64, u64);
ceil_div_impl!(ceil_div_i64, i64);
ceil_div_impl!(ceil_div_u128, u128);
ceil_div_impl!(ceil_div_i128, i128);
ceil_div_impl!(ceil_div_usize, usize);
ceil_div_impl!(ceil_div_isize, isize);
