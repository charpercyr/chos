
/// This trait is implemented for types with no destructor, and for which all bit patterns are valid.
/// Implementing this trait also means that MaybeUninit<T> and T are equivalent types.
pub unsafe trait Pod: Copy {}

macro pod($($ty:ident),* $(,)?) {
    $(
        unsafe impl Pod for $ty {}
    )*
}

pod!(
    u8, i8,
    u16, i16,
    u32, i32,
    u64, i64,
    u128, i128,
    char, bool,
);

unsafe impl<T: ?Sized> Pod for *const T {}
unsafe impl<T: ?Sized> Pod for *mut T {}

macro pod_tuple($([$($name:ident),* $(,)?]),* $(,)?) {
    $(
        unsafe impl<$($name: Pod,)*> Pod for ($($name,)*) {}
    )*
}

pod_tuple!(
    [],
    [A],
    [A, B],
    [A, B, C],
    [A, B, C, D],
    [A, B, C, D, E],
    [A, B, C, D, E, F],
    [A, B, C, D, E, F, G],
    [A, B, C, D, E, F, G, H],
    [A, B, C, D, E, F, G, H, I],
    [A, B, C, D, E, F, G, H, I, J],
    [A, B, C, D, E, F, G, H, I, J, K],
    [A, B, C, D, E, F, G, H, I, J, K, L],
);

unsafe impl<T: Pod, const N: usize> Pod for [T; N] {}
