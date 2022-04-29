#[cfg(feature = "alloc")]
use alloc::collections::VecDeque;
#[cfg(feature = "alloc")]
use alloc::vec::Vec;
use core::cell::{Cell, RefCell, UnsafeCell};
use core::marker::PhantomData;
use core::mem::MaybeUninit;

use intrusive_collections::linked_list::LinkedListOps;
use intrusive_collections::{Adapter, LinkedList};

pub trait ConstInit: Sized {
    const INIT: Self;
}

impl<T: ConstInit, const N: usize> ConstInit for [T; N] {
    const INIT: Self = [T::INIT; N];
}

impl<T: ConstInit> ConstInit for UnsafeCell<T> {
    const INIT: Self = Self::new(T::INIT);
}

impl<T: ConstInit> ConstInit for Cell<T> {
    const INIT: Self = Self::new(T::INIT);
}

impl<T: ConstInit> ConstInit for RefCell<T> {
    const INIT: Self = Self::new(T::INIT);
}

impl<T> ConstInit for MaybeUninit<T> {
    const INIT: Self = Self::uninit();
}

impl<T: ?Sized> ConstInit for PhantomData<T> {
    const INIT: Self = Self;
}

impl<T> ConstInit for Option<T> {
    const INIT: Self = None;
}

#[cfg(feature = "alloc")]
impl ConstInit for alloc::alloc::Global {
    const INIT: Self = Self;
}

impl<A: Adapter<LinkOps: LinkedListOps> + ConstInit> ConstInit for LinkedList<A> {
    const INIT: Self = LinkedList::new(ConstInit::INIT);
}

macro_rules! const_init_tuple {
    ($($i:ident),* $(,)?) => {
        impl<$($i: ConstInit,)*> ConstInit for ($($i,)*) {
            const INIT: Self = ($($i::INIT,)*);
        }
    };
}

const_init_tuple!();
const_init_tuple!(A);
const_init_tuple!(A, B);
const_init_tuple!(A, B, C);
const_init_tuple!(A, B, C, D);
const_init_tuple!(A, B, C, D, E);
const_init_tuple!(A, B, C, D, E, F);
const_init_tuple!(A, B, C, D, E, F, G);
const_init_tuple!(A, B, C, D, E, F, G, H);
const_init_tuple!(A, B, C, D, E, F, G, H, I);
const_init_tuple!(A, B, C, D, E, F, G, H, I, J);

pub trait Init: Sized {
    fn new() -> Self;
}

impl<C: ConstInit> Init for C {
    fn new() -> Self {
        Self::INIT
    }
}

#[cfg(feature = "alloc")]
impl<T> Init for Vec<T> {
    fn new() -> Self {
        Self::new()
    }
}
#[cfg(feature = "alloc")]
impl<T> Init for VecDeque<T> {
    fn new() -> Self {
        Self::new()
    }
}
