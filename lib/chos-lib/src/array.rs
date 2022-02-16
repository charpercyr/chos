mod private {
    pub trait Sealed {}
}

pub trait ConstLen: private::Sealed {
    const LEN: usize;
    #[inline]
    fn len(&self) -> usize {
        Self::LEN
    }
}

#[inline]
pub const fn const_len_of_val<T: ConstLen>(_: &T) -> usize {
    T::LEN
}

macro const_len($($n:expr => ($($t:ident),* $(,)?)),* $(,)?) {
    $(
        impl<$($t ,)*> private::Sealed for ($($t,)*) {}
        impl<$($t ,)*> ConstLen for ($($t,)*) {
            const LEN: usize = $n;
        }
    )*
}

const_len!(
    0 => (),
    1 => (T1,),
    2 => (T1, T2),
    3 => (T1, T2, T3),
    4 => (T1, T2, T3, T4),
    5 => (T1, T2, T3, T4, T5),
    6 => (T1, T2, T3, T4, T5, T6),
    7 => (T1, T2, T3, T4, T5, T6, T7),
    8 => (T1, T2, T3, T4, T5, T6, T7, T8),
    9 => (T1, T2, T3, T4, T5, T6, T7, T8, T9),
    10 => (T1, T2, T3, T4, T5, T6, T7, T8, T9, T10),
    11 => (T1, T2, T3, T4, T5, T6, T7, T8, T9, T10, T11),
    12 => (T1, T2, T3, T4, T5, T6, T7, T8, T9, T10, T11, T12),
);

impl<T, const N: usize> private::Sealed for [T; N] {}
impl<T, const N: usize> ConstLen for [T; N] {
    const LEN: usize = N;
}

pub trait Array<T>: AsRef<[T]> + AsMut<[T]> + ConstLen {}
impl<T, const N: usize> Array<T> for [T; N] {}
