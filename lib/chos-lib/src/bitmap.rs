use core::mem::{size_of, transmute};
use core::ops::{Bound, RangeBounds};
use core::slice;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct OutOfRange;

pub type Repr = usize;
pub const REPR_BITS: usize = size_of::<Repr>() * 8;

#[repr(transparent)]
#[derive(Debug)]
pub struct Bitmap {
    repr: [Repr],
}

impl<'a> From<&'a [Repr]> for &'a Bitmap {
    fn from(s: &'a [Repr]) -> Self {
        Bitmap::from_slice(s)
    }
}

impl<'a> From<&'a mut [Repr]> for &'a mut Bitmap {
    fn from(s: &'a mut [Repr]) -> Self {
        Bitmap::from_slice_mut(s)
    }
}

impl Bitmap {
    pub const fn from_slice(s: &[Repr]) -> &Self {
        unsafe { transmute(s) }
    }

    pub const fn from_slice_mut(s: &mut [Repr]) -> &mut Self {
        unsafe { transmute(s) }
    }

    pub unsafe fn from_raw_parts<'a>(data: *const Repr, len: usize) -> &'a Self {
        slice::from_raw_parts(data, len).into()
    }

    pub unsafe fn from_raw_parts_mut<'a>(data: *mut Repr, len: usize) -> &'a mut Self {
        slice::from_raw_parts_mut(data, len).into()
    }

    pub const fn get_bit(&self, i: usize) -> bool {
        let (word, bit) = word_bit(i);
        self.repr[word] & (1 << bit) != 0
    }

    pub fn checked_get_bit(&self, i: usize) -> Result<bool, OutOfRange> {
        let (word, bit) = word_bit(i);
        self.repr
            .get(word)
            .map(|&v| v & (1 << bit) != 0)
            .ok_or(OutOfRange)
    }

    pub fn set_bit(&mut self, i: usize, b: bool) {
        let (word, bit) = word_bit(i);
        if b {
            self.repr[word] |= 1 << bit;
        } else {
            self.repr[word] &= !(1 << bit);
        }
    }

    pub fn checked_set_bit(&mut self, i: usize, b: bool) -> Result<(), OutOfRange> {
        let (word, bit) = word_bit(i);
        if b {
            self.repr.get_mut(word).map(|w| *w |= 1 << bit)
        } else {
            self.repr.get_mut(word).map(|w| *w &= !(1 << bit))
        }
        .ok_or(OutOfRange)
    }

    pub fn set_all(&mut self) {
        self.set_all_to(true, ..)
    }

    pub fn unset_all(&mut self) {
        self.set_all_to(false, ..)
    }

    pub fn set_all_in<R: RangeBounds<usize>>(&mut self, r: R) {
        self.set_all_to(true, r)
    }

    pub fn unset_all_in<R: RangeBounds<usize>>(&mut self, r: R) {
        self.set_all_to(false, r)
    }

    pub const fn len(&self) -> usize {
        self.repr.len() * REPR_BITS
    }

    pub fn leading_zeroes(&self) -> u64 {
        self.leading_xs(Repr::leading_zeros)
    }

    pub fn leading_ones(&self) -> u64 {
        self.leading_xs(Repr::leading_ones)
    }

    pub fn trailing_zeroes(&self) -> u64 {
        self.trailing_xs(Repr::trailing_zeros)
    }

    pub fn trailing_ones(&self) -> u64 {
        self.trailing_xs(Repr::trailing_ones)
    }

    fn leading_xs<F: Fn(Repr) -> u32>(&self, f: F) -> u64 {
        let mut total = 0;
        self.repr
            .iter()
            .rev()
            .take_while(|&&v| {
                let lz = f(v);
                total += lz as u64;
                lz as usize == REPR_BITS
            })
            .for_each(|_| ());
        total
    }

    fn trailing_xs<F: Fn(Repr) -> u32>(&self, f: F) -> u64 {
        let mut total = 0;
        self.repr
            .iter()
            .take_while(|&&v| {
                let lz = f(v);
                total += lz as u64;
                lz as usize == REPR_BITS
            })
            .for_each(|_| ());
        total
    }

    fn set_all_to<R: RangeBounds<usize>>(&mut self, v: bool, r: R) {
        let start = match r. start_bound() {
            Bound::Included(&i) => i,
            Bound::Excluded(&0) => panic!("Invalid bounds"),
            Bound::Excluded(&i) => i - 1,
            Bound::Unbounded => 0,
        };
        let end = match r.end_bound() {
            Bound::Included(&i) => i + 1,
            Bound::Excluded(&i) => i,
            Bound::Unbounded => self.len(),
        };
        let (mut start_word, start_bit) = word_bit(start);
        let (end_word, end_bit) = word_bit(end);

        if start_word == end_word {
            let mask = !((1 << start_bit) - 1) & ((1 << end_bit) - 1);
            if v {
                self.repr[start_word] |= mask;
            } else {
                self.repr[start_word] &= !mask;
            }
        } else {
            if start_bit != 0 {
                let mask = (1 << start_bit) - 1;
                if v {
                    self.repr[start_word] |= !mask;
                } else {
                    self.repr[start_word] &= mask;
                }
                start_word += 1;
            }
            for i in start_word..end_word {
                if v {
                    self.repr[i] = !0;
                } else {
                    self.repr[i] = 0;
                }
            }
            if end_bit != 0 {
                let mask = (1 << end_bit) - 1;
                if v  {
                    self.repr[end_word] |= mask;
                } else {
                    self.repr[end_word] &= !mask;
                }
            }
        }
    }
}

const fn word_bit(idx: usize) -> (usize, usize) {
    (idx / REPR_BITS, idx % REPR_BITS)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_get_bit() {
        let s = [5];
        let b = Bitmap::from_slice(&s);
        assert_eq!(b.get_bit(0), true);
        assert_eq!(b.get_bit(1), false);
        assert_eq!(b.get_bit(2), true);
        for i in 3..64 {
            assert_eq!(b.get_bit(i), false);
        }
    }

    #[test]
    fn test_set_bit() {
        let mut s = [0];
        let b = Bitmap::from_slice_mut(&mut s);
        b.set_bit(0, true);
        b.set_bit(2, true);
        assert_eq!(s, [5]);
    }

    #[test]
    fn test_get_bit_multiple() {
        let s = [1, 2, 3, 4];
        let b = Bitmap::from_slice(&s);
        assert_eq!(b.get_bit(0), true);
        assert_eq!(b.get_bit(65), true);
        assert_eq!(b.get_bit(128), true);
        assert_eq!(b.get_bit(129), true);
        assert_eq!(b.get_bit(194), true);
    }

    #[test]
    fn test_set_bit_multiple() {
        let mut s = [0; 4];
        let b = Bitmap::from_slice_mut(&mut s);
        b.set_bit(0, true);
        b.set_bit(65, true);
        b.set_bit(128, true);
        b.set_bit(129, true);
        b.set_bit(194, true);
        assert_eq!(s, [1, 2, 3, 4]);
    }

    #[test]
    fn test_leading_zeroes() {
        let s = [0x1, 0x0, 0x0, 0x0];
        let b = Bitmap::from_slice(&s);
        assert_eq!(b.leading_zeroes(), 255);
    }

    #[test]
    fn test_leading_ones() {
        let s = [!0x1, !0x0, !0x0, !0x0];
        let b = Bitmap::from_slice(&s);
        assert_eq!(b.leading_ones(), 255);
    }

    #[test]
    fn test_trailing_zeroes() {
        let s = [0x0, 0x0, 0x0, 1 << 63];
        let b = Bitmap::from_slice(&s);
        assert_eq!(b.trailing_zeroes(), 255);
    }

    #[test]
    fn test_trailing_ones() {
        let s = [!0x0, !0x0, !0x0, !(1 << 63)];
        let b = Bitmap::from_slice(&s);
        assert_eq!(b.trailing_ones(), 255);
    }

    #[test]
    fn test_len() {
        let s = [0; 4];
        let b = Bitmap::from_slice(&s);
        assert_eq!(b.len(), s.len() * REPR_BITS);
    }

    #[test]
    fn set_all() {
        let mut s = [0; 4];
        let b = Bitmap::from_slice_mut(&mut s);
        b.set_all();
        assert_eq!(s, [!0; 4]);
    }

    #[test]
    fn set_all_in() {
        let mut s = [0; 4];
        let b = Bitmap::from_slice_mut(&mut s);
        b.set_all_in(1..191);
        assert_eq!(s, [
            0xffff_ffff_ffff_fffe,
            0xffff_ffff_ffff_ffff,
            0x7fff_ffff_ffff_ffff,
            0x0,
        ]);
    }

    #[test]
    fn set_all_in_end_unbounded() {
        let mut s = [0; 4];
        let b = Bitmap::from_slice_mut(&mut s);
        b.set_all_in(1..);
        assert_eq!(s, [
            0xffff_ffff_ffff_fffe,
            0xffff_ffff_ffff_ffff,
            0xffff_ffff_ffff_ffff,
            0xffff_ffff_ffff_ffff,
        ]);
    }

    #[test]
    fn set_all_in_start_unbounded() {
        let mut s = [0; 4];
        let b = Bitmap::from_slice_mut(&mut s);
        b.set_all_in(..255);
        assert_eq!(s, [
            0xffff_ffff_ffff_ffff,
            0xffff_ffff_ffff_ffff,
            0xffff_ffff_ffff_ffff,
            0x7fff_ffff_ffff_ffff,
        ]);
    }

    #[test]
    fn set_all_in_unbounded() {
        let mut s = [0; 4];
        let b = Bitmap::from_slice_mut(&mut s);
        b.set_all_in(..);
        assert_eq!(s, [
            0xffff_ffff_ffff_ffff,
            0xffff_ffff_ffff_ffff,
            0xffff_ffff_ffff_ffff,
            0xffff_ffff_ffff_ffff,
        ]);
    }

    #[test]
    fn set_all_in_same_word() {
        let mut s = [0; 4];
        let b = Bitmap::from_slice_mut(&mut s);
        b.set_all_in(65..68);
        assert_eq!(s, [
            0x0,
            0xe,
            0x0,
            0x0
        ]);
    }

    #[test]
    fn set_all_in_same_word_start_unbounded() {
        let mut s = [0];
        let b = Bitmap::from_slice_mut(&mut s);
        b.set_all_in(..63);
        assert_eq!(s, [0x7fff_ffff_ffff_ffff]);
    }

    #[test]
    fn set_all_in_same_word_end_unbounded() {
        let mut s = [0];
        let b = Bitmap::from_slice_mut(&mut s);
        b.set_all_in(1..);
        assert_eq!(s, [0xffff_ffff_ffff_fffe]);
    }

    #[test]
    fn unset_all() {
        let mut s = [!0; 4];
        let b = Bitmap::from_slice_mut(&mut s);
        b.unset_all();
        assert_eq!(s, [0; 4]);
    }

    #[test]
    fn unset_all_in() {
        let mut s = [!0; 4];
        let b = Bitmap::from_slice_mut(&mut s);
        b.unset_all_in(1..191);
        assert_eq!(s, [
            !0xffff_ffff_ffff_fffe,
            !0xffff_ffff_ffff_ffff,
            !0x7fff_ffff_ffff_ffff,
            !0x0,
        ]);
    }

    #[test]
    fn unset_all_in_end_unbounded() {
        let mut s = [!0; 4];
        let b = Bitmap::from_slice_mut(&mut s);
        b.unset_all_in(1..);
        assert_eq!(s, [
            !0xffff_ffff_ffff_fffe,
            !0xffff_ffff_ffff_ffff,
            !0xffff_ffff_ffff_ffff,
            !0xffff_ffff_ffff_ffff,
        ]);
    }

    #[test]
    fn unset_all_in_start_unbounded() {
        let mut s = [!0; 4];
        let b = Bitmap::from_slice_mut(&mut s);
        b.unset_all_in(..255);
        assert_eq!(s, [
            !0xffff_ffff_ffff_ffff,
            !0xffff_ffff_ffff_ffff,
            !0xffff_ffff_ffff_ffff,
            !0x7fff_ffff_ffff_ffff,
        ]);
    }

    #[test]
    fn unset_all_in_unbounded() {
        let mut s = [!0; 4];
        let b = Bitmap::from_slice_mut(&mut s);
        b.unset_all_in(..);
        assert_eq!(s, [
            !0xffff_ffff_ffff_ffff,
            !0xffff_ffff_ffff_ffff,
            !0xffff_ffff_ffff_ffff,
            !0xffff_ffff_ffff_ffff,
        ]);
    }

    #[test]
    fn unset_all_in_same_word() {
        let mut s = [!0; 4];
        let b = Bitmap::from_slice_mut(&mut s);
        b.unset_all_in(65..68);
        assert_eq!(s, [
            !0x0,
            !0xe,
            !0x0,
            !0x0
        ]);
    }

    #[test]
    fn unset_all_in_same_word_start_unbounded() {
        let mut s = [!0];
        let b = Bitmap::from_slice_mut(&mut s);
        b.unset_all_in(..63);
        assert_eq!(s, [!0x7fff_ffff_ffff_ffff]);
    }

    #[test]
    fn unset_all_in_same_word_end_unbounded() {
        let mut s = [!0];
        let b = Bitmap::from_slice_mut(&mut s);
        b.unset_all_in(1..);
        assert_eq!(s, [!0xffff_ffff_ffff_fffe]);
    }
}
