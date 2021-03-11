#[macro_export]
macro_rules! include_asm {
    ($($path:expr),* $(,)?) => {
        $(
            global_asm!(include_str!($path));
        )*
    }
}

#[macro_export]
macro_rules! offset_of {
    ($field:ident, $container:ty) => {{
        #[inline(always)]
        fn offset_of() -> usize {
            unsafe {
                use core::mem::MaybeUninit;
                let container: MaybeUninit<$container> = MaybeUninit::uninit();
                let container = container.get_ref();
                let field = &container.$field;
                let container = container as *const _;
                let container = container as *const u8;
                let field = field as *const _;
                let field = field as *const u8;
                field.offset_from(container) as usize
            }
        }
        offset_of()
    }};
}

#[macro_export]
macro_rules! container_of {
    ($ptr:expr, $field:ident, $container:ty) => {{
        #[inline(always)]
        unsafe fn container_of() -> *const $container {
            let ptr = $ptr as *const u8;
            let ptr = ptr.sub(offset_of!($field, $container));
            ptr as *const $container
        }
        container_of()
    }};
}
