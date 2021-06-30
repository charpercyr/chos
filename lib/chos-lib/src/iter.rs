
pub trait IteratorExt: Iterator {
    /// Combined min and max operation on a iterator of tuples
    /// Finds the min of the first element, and finds the max of the second element
    fn min_max<A, B>(mut self) -> Option<Self::Item>
        where
            Self: Sized + Iterator<Item = (A, B)>,
            A: Ord,
            B: Ord,
    {
        let mut mm = self.next()?;
        for (a, b) in self {
            if a < mm.0 {
                mm.0 = a;
            }
            if b > mm.1 {
                mm.1 = b;
            }
        }
        Some(mm)
    }
}
impl<I: Iterator + ?Sized> IteratorExt for I {}
