
#[macro_export]
macro_rules! offset_of{
    ($field:ident, $container:ty) => {{
        #[inline(always)]
        fn offset_of() -> usize {
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
        }
        offset_of()
    }};
}

#[macro_export]
macro_rules! container_of {
    ($ptr:expr, $field:ident, $container:ty) => {{
        let ptr = $ptr as *const u8;
        let ptr = ptr.sub($crate::offset_of!($field, $container));
        ptr as *const $container
    }};
}

#[macro_export]
macro_rules! intrusive_adapter {
    ($(#[$attr:meta])* $(pub $(($($vis:tt)*))?)? struct $name:ident = $ptr:ty : $value:ty { $field:ident : $fty:ty }) => {
        $(#[$attr])*
        $(pub $(($($vis)*))*)* struct $name;
        impl $crate::intrusive::Adapter for $name {
            type Value = $value;
            type Pointer = $ptr;
            type Link = $fty;

            unsafe fn get_link(&self, value: *const Self::Value) -> *const Self::Link {
                &(*value).$field
            }

            unsafe fn get_value(&self, link: *const Self::Link) -> *const Self::Value {
                $crate::container_of!(link, $field, $value)
            }
        }
    };
}