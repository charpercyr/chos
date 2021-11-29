#[macro_export]
macro_rules! offset_of {
    ($field:ident, $container:ty) => {
        #[allow(unused_unsafe)]
        unsafe {
            use core::mem::MaybeUninit;
            let container: MaybeUninit<$container> = MaybeUninit::uninit();
            let container = container.assume_init_ref();
            let field = &container.$field;
            let container = container as *const _;
            let container = container as *const u8;
            let field = field as *const _;
            let field = field as *const u8;
            field.offset_from(container) as usize
        }
    };
}

#[macro_export]
macro_rules! container_of {
    ($ptr:expr, $field:ident, $container:ty) => {{
        let ptr = $ptr as *const u8;
        let ptr = ptr.sub($crate::offset_of!($field, $container));
        ptr as *mut $container
    }};
}

#[macro_export]
macro_rules! match_arch {
    ($($arch:expr => $amod:ident),* $(,)?) => {
        $(
            $crate::cfg_if::cfg_if! {
                if #[cfg(target_arch = $arch)] {
                    pub mod $amod;
                    pub use self::$amod::*;
                }
            }
        )*
    };
}

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
