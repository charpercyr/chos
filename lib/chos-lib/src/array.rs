use core::ops::DerefMut;

pub trait Array<T>: DerefMut<Target = [T]> + Sized {
    fn len(&self) -> usize;
}
