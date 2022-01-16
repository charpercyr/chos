macro domains ($($name:ident = $value:expr),* $(,)?) {
    $(
        paste::item! {
            pub struct [<__ $name:camel>];
            pub const $name: [<__ $name:camel>] = [<__ $name:camel>];
            impl chos_lib::log::Domain for [<__ $name:camel>] {
                #[inline]
                fn name(&self) -> &str {
                    stringify!($name)
                }
                #[inline]
                fn enabled(&self) -> bool {
                    $value
                }
            }
        }
    )*
}

domains!{
    PALLOC = false,
    GLOBAL_ALLOC = false,
    SLAB_ALLOC = false,
}
