pub mod hash_table;
pub mod list;
mod pointer;
pub use pointer::*;

pub trait Adapter {
    type Value: ?Sized;
    type Link: LinkOps;
    type Pointer: PointerOps<Target = Self::Value, Metadata = <Self::Link as LinkOps>::Metadata>;

    unsafe fn get_link(&self, value: *const Self::Value) -> *const Self::Link;
    unsafe fn get_value(&self, link: *const Self::Link) -> *const Self::Value;
}

pub trait KeyAdapter: Adapter {
    type Key<'a>;
    fn get_key<'a>(&self, value: &'a Self::Value) -> Self::Key<'a>;
}

pub trait LinkOps {
    type Metadata;

    fn acquire(&self) -> bool;
    fn release(&self);

    unsafe fn set_meta(&self, meta: Self::Metadata);
    unsafe fn take_meta(&self) -> Self::Metadata;
}

#[macro_export]
macro_rules! intrusive_adapter {
    (
        $(#[$attr:meta])*
        $(pub $(($($vis:tt)*))?)?
        struct $name:ident $(<$($lif:lifetime),* $(,)?>)?
        =
        $ptr:ty : $value:ty
        { $field:ident : $fty:ty }
        $(where $($where:tt)*)?
    ) => {
        $(#[$attr])*
        $(pub $(($($vis)*))*)* struct $name $(<$($lif,)*>)* (core::marker::PhantomData<($($(& $lif (),)*)*)>) $(where $($where)*)*;
        impl $(<$($lif,)*>)* $name $(<$($lif,)*>)* $(where $($where)*)* {
            pub const fn new() -> Self {
                Self(core::marker::PhantomData)
            }
        }
        impl $(<$($lif,)*>)* $crate::intrusive::Adapter for $name $(<$($lif,)*>)* $(where $($where)*)* {
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
        impl $(<$($lif,)*>)* $crate::init::ConstInit for $name $(<$($lif,)*>)* $(where $($where)*)* {
            const INIT: Self = Self::new();
        }
    }
}
