#[macro_export]
macro_rules! include_asm {
    ($($path:expr),* $(,)?) => {
        $(
            global_asm!(concat!(
                ".att_syntax\n",
                include_str!($path),
            ));
        )*
    };
}
