pub trait WriteAccess: private::Sealed {}
pub trait ReadAccess: private::Sealed {}

pub struct NoAccess(());
pub struct WriteOnly(());
pub struct ReadOnly(());
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
