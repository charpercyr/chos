use core::ops;

const NS_PER_SEC: u32 = 1_000_000_000;

#[derive(Copy, Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Default)]
pub struct Duration64 {
    secs: u64,
    nanos: u32,
}

impl Duration64 {
    pub const fn from_secs(secs: u64) -> Self {
        Self { secs, nanos: 0 }
    }

    pub const fn from_millis(millis: u64) -> Self {
        Self {
            secs: millis / 1000,
            nanos: ((millis % 1000) * 1_000_000) as u32,
        }
    }

    pub const fn from_micros(micros: u64) -> Self {
        Self {
            secs: micros / 1_000_000,
            nanos: ((micros % 1_000_000) * 1000) as u32,
        }
    }

    pub const fn from_nanos(nanos: u64) -> Self {
        Self {
            secs: nanos / 1_000_000_000,
            nanos: (nanos % 1_000_000_000) as u32,
        }
    }

    pub const fn as_nanos(self) -> u64 {
        self.secs * 1_000_000_000 + self.nanos as u64
    }

    pub const fn as_millis(self) -> u64 {
        self.secs * 1_000_000 + self.nanos as u64 / 1_000
    }

    pub const fn as_micros(self) -> u64 {
        self.secs * 1_000 + self.nanos as u64 / 1_000_000
    }

    pub const fn as_secs(self) -> u64 {
        self.secs
    }

    pub fn checked_add(self, rhs: Self) -> Option<Self> {
        let mut secs = self.secs.checked_add(rhs.secs)?;
        let mut nanos = self.nanos + rhs.nanos;
        if nanos >= NS_PER_SEC {
            secs = secs.checked_add(1)?;
            nanos -= NS_PER_SEC
        }
        debug_assert!(nanos < NS_PER_SEC);
        Some(Self { secs, nanos })
    }

    pub fn checked_sub(self, rhs: Self) -> Option<Self> {
        let mut secs = self.secs.checked_sub(rhs.secs)?;
        let nanos = if self.nanos >= rhs.nanos {
            self.nanos - rhs.nanos
        } else {
            secs = secs.checked_sub(1)?;
            self.nanos + NS_PER_SEC - rhs.nanos
        };
        debug_assert!(nanos < NS_PER_SEC);
        Some(Self {secs, nanos })
    }
}

impl ops::Add for Duration64 {
    type Output = Self;
    fn add(self, rhs: Self) -> Self {
        self.checked_add(rhs).expect("Duration overflow")
    }
}

impl ops::Sub for Duration64 {
    type Output = Self;
    fn sub(self, rhs: Self) -> Self {
        self.checked_sub(rhs).expect("Duration overflow")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn from_secs() {
        let d = Duration64::from_secs(1);
        assert_eq!(d.secs, 1);
        assert_eq!(d.nanos, 0);
    }

    #[test]
    fn from_millis() {
        let d = Duration64::from_millis(1100);
        assert_eq!(d.secs, 1);
        assert_eq!(d.nanos, 100_000_000);
    }

    #[test]
    fn from_micros() {
        let d = Duration64::from_micros(1_100_000);
        assert_eq!(d.secs, 1);
        assert_eq!(d.nanos, 100_000_000);
    }

    #[test]
    fn from_nanos() {
        let d = Duration64::from_nanos(1_100_000_000);
        assert_eq!(d.secs, 1);
        assert_eq!(d.nanos, 100_000_000);
    }

    #[test]
    fn add() {
        let d = Duration64::from_secs(1) + Duration64::from_nanos(1);
        assert_eq!(d.secs, 1);
        assert_eq!(d.nanos, 1);

        let d = Duration64::from_millis(550) + Duration64::from_millis(550);
        assert_eq!(d.secs, 1);
        assert_eq!(d.nanos, 100_000_000);
    }

    #[test]
    fn sub() {
        let d = Duration64::from_nanos(100) - Duration64::from_nanos(1);
        assert_eq!(d.secs, 0);
        assert_eq!(d.nanos, 99);

        let d = Duration64::from_secs(2) - Duration64::from_nanos(1);
        assert_eq!(d.secs, 1);
        assert_eq!(d.nanos, 999_999_999);
    }
}