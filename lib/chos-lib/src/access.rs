pub trait WriteAccess: private::Sealed {}
pub trait ReadAccess: private::Sealed {}

#[derive(Copy, Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct NoAccess(());

#[derive(Copy, Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct WriteOnly(());

#[derive(Copy, Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ReadOnly(());

#[derive(Copy, Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ReadWrite(());

impl WriteAccess for WriteOnly {}
impl WriteAccess for ReadWrite {}
impl ReadAccess for ReadOnly {}
impl ReadAccess for ReadWrite {}

mod private {
    use super::*;
    pub trait Sealed {}

    impl Sealed for NoAccess {}
    impl Sealed for WriteOnly {}
    impl Sealed for ReadOnly {}
    impl Sealed for ReadWrite {}
}
