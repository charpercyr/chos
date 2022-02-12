pub mod hash_table;

use core::marker::PhantomData;

use crate::init::ConstInit;

pub struct DefaultPointerOps<P>(PhantomData<P>);

impl<P> Clone for DefaultPointerOps<P> {
    fn clone(&self) -> Self {
        *self
    }
}
impl<P> Copy for DefaultPointerOps<P> {}

impl<P> DefaultPointerOps<P> {
    pub const fn new() -> Self {
        Self(PhantomData)
    }
}

impl<P> ConstInit for DefaultPointerOps<P> {
    const INIT: Self = Self::new();
}

#[macro_export]
macro_rules! intrusive_adapter {
    (@impl
        $(#[$attr:meta])* $vis:vis $name:ident ($($args:tt),*)
        = $pointer:ty: $value:path { $field:ident: $link:ty } $($where_:tt)*
    ) => {
        #[allow(explicit_outlives_requirements)]
        $(#[$attr])*
        $vis struct $name<$($args),*> $($where_)* {
            link_ops: <$link as intrusive_collections::DefaultLinkOps>::Ops,
            pointer_ops: $crate::intrusive::DefaultPointerOps<$pointer>,
        }
        unsafe impl<$($args),*> Send for $name<$($args),*> $($where_)* {}
        unsafe impl<$($args),*> Sync for $name<$($args),*> $($where_)* {}
        impl<$($args),*> Copy for $name<$($args),*> $($where_)* {}
        impl<$($args),*> Clone for $name<$($args),*> $($where_)* {
            #[inline]
            fn clone(&self) -> Self {
                *self
            }
        }
        impl<$($args),*> Default for $name<$($args),*> $($where_)* {
            #[inline]
            fn default() -> Self {
                Self::NEW
            }
        }
        #[allow(dead_code)]
        impl<$($args),*> $name<$($args),*> $($where_)* {
            pub const NEW: Self = $name {
                link_ops: <$link as intrusive_collections::DefaultLinkOps>::NEW,
                pointer_ops: $crate::intrusive::DefaultPointerOps::<$pointer>::new(),
            };
            #[inline]
            pub const fn new() -> Self {
                Self::NEW
            }
        }
        #[allow(dead_code, unsafe_code)]
        unsafe impl<$($args),*> intrusive_collections::Adapter for $name<$($args),*> $($where_)* {
            type LinkOps = <$link as intrusive_collections::DefaultLinkOps>::Ops;
            type PointerOps = $crate::intrusive::DefaultPointerOps<$pointer>;

            #[inline]
            unsafe fn get_value(&self, link: <Self::LinkOps as intrusive_collections::LinkOps>::LinkPtr) -> *const <Self::PointerOps as intrusive_collections::PointerOps>::Value {
                intrusive_collections::container_of!(link.as_ptr(), $value, $field)
            }
            #[inline]
            unsafe fn get_link(&self, value: *const <Self::PointerOps as intrusive_collections::PointerOps>::Value) -> <Self::LinkOps as intrusive_collections::LinkOps>::LinkPtr {
                // We need to do this instead of just accessing the field directly
                // to strictly follow the stack borrow rules.
                let ptr = (value as *const u8).add(intrusive_collections::offset_of!($value, $field));
                core::ptr::NonNull::new_unchecked(ptr as *mut _)
            }
            #[inline]
            fn link_ops(&self) -> &Self::LinkOps {
                &self.link_ops
            }
            #[inline]
            fn link_ops_mut(&mut self) -> &mut Self::LinkOps {
                &mut self.link_ops
            }
            #[inline]
            fn pointer_ops(&self) -> &Self::PointerOps {
                &self.pointer_ops
            }
        }

        impl<$($args),*> $crate::init::ConstInit for $name<$($args),*> $($where_)* {
            const INIT: Self = Self::new();
        }
    };
    (@find_generic
        $(#[$attr:meta])* $vis:vis $name:ident ($($prev:tt)*) > $($rest:tt)*
    ) => {
        $crate::intrusive_adapter!(@impl
            $(#[$attr])* $vis $name ($($prev)*) $($rest)*
        );
    };
    (@find_generic
        $(#[$attr:meta])* $vis:vis $name:ident ($($prev:tt)*) $cur:tt $($rest:tt)*
    ) => {
        $crate::intrusive_adapter!(@find_generic
            $(#[$attr])* $vis $name ($($prev)* $cur) $($rest)*
        );
    };
    (@find_if_generic
        $(#[$attr:meta])* $vis:vis $name:ident < $($rest:tt)*
    ) => {
        $crate::intrusive_adapter!(@find_generic
            $(#[$attr])* $vis $name () $($rest)*
        );
    };
    (@find_if_generic
        $(#[$attr:meta])* $vis:vis $name:ident $($rest:tt)*
    ) => {
        $crate::intrusive_adapter!(@impl
            $(#[$attr])* $vis $name () $($rest)*
        );
    };
    ($(#[$attr:meta])* $vis:vis $name:ident $($rest:tt)*) => {
        $crate::intrusive_adapter!(@find_if_generic
            $(#[$attr])* $vis $name $($rest)*
        );
    };
}
