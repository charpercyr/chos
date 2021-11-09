use core::hint::unreachable_unchecked;

pub trait CeilDiv {
    fn ceil_div(self, rhs: Self) -> Self;

    fn align_up(self, align: Self) -> Self;
}

macro_rules! ceil_div_impl {
    ($($ty:ty),* $(,)?) => {
        $(
            paste::item! {
                impl CeilDiv for $ty {
                    #[inline]
                    fn ceil_div(self, rhs: Self) -> Self {
                        [<ceil_div $ty>](self, rhs)
                    }

                    #[inline]
                    fn align_up(self, align: Self) -> Self {
                        [<align_up $ty>](self, align)
                    }
                }

                pub const fn [<ceil_div $ty>](a: $ty, b: $ty) -> $ty {
                    (a + b - 1) / b
                }

                pub const fn [<align_up $ty>](v: $ty, align: $ty) -> $ty {
                    [<ceil_div $ty>](v, align) * align
                }
            }
        )*
    };
}

ceil_div_impl!(u8, i8, u16, i16, u32, i32, u64, i64, u128, i128, usize, isize,);

pub const fn log2u64(value: u64) -> u32 {
    assert!(value != 0, "Cannot calculate log of 0");
    63 - value.leading_zeros()
}

pub const fn ceil_log2u64(value: u64) -> u32 {
    assert!(value != 0, "Cannot calculate log of 0");
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

pub trait IntSplit {
    type Split;
    fn split(self) -> (Self::Split, Self::Split);
    fn join(h: Self::Split, l: Self::Split) -> Self;
}

macro_rules! impl_int_split {
    ($t:ty, $s:ty) => {
        impl IntSplit for $t {
            type Split = $s;
            fn split(self) -> ($s, $s) {
                const SHIFT: usize = core::mem::size_of::<$s>() * 8;
                ((self >> SHIFT) as $s, (self & ((1 << SHIFT) - 1)) as $s)
            }
            fn join(h: $s, l: $s) -> $t {
                const SHIFT: usize = core::mem::size_of::<$s>() * 8;
                ((h as $t) << SHIFT) | (l as $t)
            }
        }
    };
}
impl_int_split!(u128, u64);
impl_int_split!(u64, u32);
impl_int_split!(u32, u16);
impl_int_split!(u16, u8);

#[cfg(target_pointer_width = "32")]
impl_int_split!(usize, u16);
#[cfg(target_pointer_width = "64")]
impl_int_split!(usize, u32);
