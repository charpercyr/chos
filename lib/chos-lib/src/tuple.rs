mod private {
    pub trait Sealed {}
}

pub trait TupleLen: private::Sealed {
    const LEN: usize;
    fn len(&self) -> usize {
        Self::LEN
    }
}

macro tuple_len($($n:expr => ($($t:ident),* $(,)?)),* $(,)?) {
    $(
        impl<$($t ,)*> private::Sealed for ($($t,)*) {}
        impl<$($t ,)*> TupleLen for ($($t,)*) {
            const LEN: usize = $n;
        }
    )*
}

tuple_len!(
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
impl<T, const N: usize> TupleLen for [T; N] {
    const LEN: usize = N;
}
