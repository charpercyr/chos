use core::{fmt, ops};

#[derive(Clone, Copy, Default, PartialEq, Eq)]
pub struct Cpumask(u64);

impl Cpumask {
    pub const fn from_raw(raw: u64) -> Self {
        Self(raw)
    }

    pub const fn empty() -> Self {
        Self(0)
    }

    pub const fn for_cpu(n: u8) -> Self {
        Self(1 << n)
    }

    pub const fn difference(self, rhs: Self) -> Self {
        Self(self.0 & !rhs.0)
    }

    pub const fn union(self, rhs: Self) -> Self {
        Self(self.0 | rhs.0)
    }

    pub const fn intersection(self, rhs: Self) -> Self {
        Self(self.0 & rhs.0)
    }

    pub const fn iter(self) -> CpumaskIter {
        CpumaskIter(self.0)
    }

    pub const fn raw(self) -> u64 {
        self.0
    }

    pub const fn contains(self, rhs: Self) -> bool {
        (self.0 & rhs.0) == rhs.0
    }
}

impl fmt::Debug for Cpumask {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.write_str("Cpumask")?;
        f.debug_list().entries(self.iter()).finish()
    }
}

impl ops::BitAnd for Cpumask {
    type Output = Self;
    fn bitand(self, rhs: Self) -> Self {
        self.intersection(rhs)
    }
}

impl ops::BitAndAssign for Cpumask {
    fn bitand_assign(&mut self, rhs: Self) {
        *self = self.intersection(rhs);
    }
}

impl ops::BitOr for Cpumask {
    type Output = Self;
    fn bitor(self, rhs: Self) -> Self {
        self.union(rhs)
    }
}

impl ops::BitOrAssign for Cpumask {
    fn bitor_assign(&mut self, rhs: Self) {
        *self = self.union(rhs);
    }
}

impl ops::Sub for Cpumask {
    type Output = Self;
    fn sub(self, rhs: Self) -> Self {
        self.difference(rhs)
    }
}

impl ops::SubAssign for Cpumask {
    fn sub_assign(&mut self, rhs: Self) {
        *self = self.difference(rhs);
    }
}

impl IntoIterator for Cpumask {
    type Item = u8;
    type IntoIter = CpumaskIter;
    fn into_iter(self) -> Self::IntoIter {
        self.iter()
    }
}

#[derive(Clone)]
pub struct CpumaskIter(u64);
impl Iterator for CpumaskIter {
    type Item = u8;
    fn next(&mut self) -> Option<u8> {
        (self.0 != 0).then(|| {
            let bit = self.0.trailing_zeros();
            self.0 &= !(1 << bit);
            bit as u8
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::prelude::v1::*;
    #[test]
    fn contains() {
        let sup = Cpumask::from_raw(0b1110_0000);
        assert!(sup.contains(Cpumask::from_raw(0b1100_0000)));
        assert!(sup.contains(Cpumask::from_raw(0b1000_0000)));
        assert!(!sup.contains(Cpumask::from_raw(0b1111_0000)));
        assert!(!sup.contains(Cpumask::from_raw(0b0000_0001)));
    }
}
