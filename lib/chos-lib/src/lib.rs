#![no_std]

#![feature(allocator_api)]
#![feature(associated_type_bounds)]
#![feature(bool_to_option)]
#![feature(core_intrinsics)]
#![feature(const_fn_trait_bound)]
#![feature(const_fn_transmute)]
#![feature(const_mut_refs)]
#![feature(const_panic)]
#![feature(const_unreachable_unchecked)]
#![feature(decl_macro)]
#![feature(dropck_eyepatch)]

#[cfg(feature = "alloc")]
extern crate alloc;

mod macros;
pub use macros::*;

mod either;
pub use either::*;

pub mod int;

pub mod intrusive;

pub mod iter;

pub mod pool;

pub mod spin;

pub mod str;

pub mod stride;

mod volatile;
pub use volatile::*;

pub use chos_lib_macros::forward_fmt;

#[cfg(test)]
extern crate std;

#[cfg(test)]
mod tests {
    use std::prelude::v1::*;
    #[test]
    fn my_test() {
        let v = Vec::<usize>::new();
        assert!(v.len() == 0);
    }
}