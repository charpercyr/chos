pub trait ConstInit: Sized {
    const INIT: Self;
}

impl ConstInit for () {
    const INIT: Self = ();
}

impl<T: ConstInit, const N: usize> ConstInit for [T; N] {
    const INIT: Self = [T::INIT; N];
}

#[cfg(feature = "alloc")]
impl ConstInit for alloc::alloc::Global {
    const INIT: Self = Self;
}
