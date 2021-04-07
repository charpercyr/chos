
pub use chos_lib_macros::bitfield;

pub trait FieldCast<R>: Sized {
    fn from_repr(v: R) -> Self;
    fn into_repr(self) -> R;
}

impl<T> FieldCast<T> for T {
    fn from_repr(v: T) -> T {
        v
    }
    fn into_repr(self) -> T {
        self
    }
}

macro_rules! impl_field_cast {
    ($([$from:ident, $to:ident]),* $(,)?) => {
        $(
            impl FieldCast<$from> for $to {
                fn from_repr(v: $from) -> $to {
                    v as $to
                }
                fn into_repr(self) -> $from {
                    self as $from
                }
            }
        )*
    };
}
impl_field_cast!(
    [u16, u8],
    [u32, u8],      [u32, u16],
    [u64, u8],      [u64, u16],     [u64, u32],
    [u128, u8],     [u128, u16],    [u128, u32],    [u128, u64],
);
#[cfg(target_pointer_width = "16")]
impl_field_cast!(
    [usize, u8], [usize, u16],
    [u16, usize], [u32, usize], [u64, usize], [u128, usize],
);
#[cfg(target_pointer_width = "32")]
impl_field_cast!(
    [usize, u8], [usize, u16], [usize, u32],
    [u32, usize], [u64, usize], [u128, usize],
);
#[cfg(target_pointer_width = "64")]
impl_field_cast!(
    [usize, u8], [usize, u16], [usize, u32], [usize, u64],
    [u64, usize], [u128, usize],
);

macro_rules! impl_field_to_bool {
    ($($ty:ident),* $(,)?) => {
        $(
            impl FieldCast<$ty> for bool {
                fn from_repr(v: $ty) -> Self {
                    v != 0
                }
                fn into_repr(self) -> $ty {
                    self as $ty
                }
            }
        )*
    };
}
impl_field_to_bool!(u8, u16, u32, u64, u128, usize);

pub trait Bitfield {
    fn get_bits(&self, from: usize, to: usize) -> Self;
}

pub trait BitfieldMut: Bitfield {
    fn set_bits(&mut self, from: usize, to: usize, bits: Self);
}

macro_rules! impl_bitfield_repr {
    (@mask $int:ident, $from:expr, $to:expr) => {{
        let bits = $to - $from + 1;
        (((1 as $int) << bits) - 1) << $from 
    }};
    ($($int:ident),* $(,)?) => {
        $(
            impl Bitfield for $int {
                fn get_bits(&self, from: usize, to: usize) -> Self {
                    let from = from as u32;
                    let to = to as u32;
                    let mask = impl_bitfield_repr!(@mask $int, from, to);
                    (*self & mask) >> from
                }
            }
            impl BitfieldMut for $int {
                fn set_bits(&mut self, from: usize, to: usize, bits: Self) {
                    let from = from as u32;
                    let to = to as u32;
                    let mask = impl_bitfield_repr!(@mask $int, from, to);
                    *self = (*self & !mask) | ((bits << from) & mask);
                }
            }
        )*
    };
}

impl_bitfield_repr!(
    u8,
    u16,
    u32,
    u64,
    u128,
    usize,
);

#[macro_export]
macro_rules! field_enum {
    (
        @field
        $name:ident,
        $repr:ident,
        $(
            $field:ident = $value:expr
        ),* $(,)?
    ) => {
        fn from_repr(v: $repr) -> Self {
            match v {
                $(
                    $value => Self::$field,
                )*
                _ => panic!(concat!("Invalid repr for ", stringify!($name))),
            }
        }
        fn into_repr(self) -> $repr {
            match self {
                $(
                    Self::$field => $value,
                )*
            }
        }
    };

    (
        $(
            $(#[$attr:meta])*
            $(pub $(($($vis:tt)*))?)?
            enum $name:ident ($repr:ident) {
                $(
                    $field:ident = $value:expr
                ),* $(,)?
            }
        )*
    ) => {
        $(
            #[repr($repr)]
            $(#[$attr])*
            $(pub $(($($vis)*))*)*
            enum $name {
                $($field = $value,)*
            }
    
            impl FieldCast<$repr> for $name {
                $crate::field_enum!(@field $name, $repr, $($field = $value,)*);
            }
        )*
    };
}