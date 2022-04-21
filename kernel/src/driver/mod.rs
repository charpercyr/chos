pub mod block;

pub trait Driver: Send + Sync {}

pub trait Device: Send + Sync {}

pub fn register_driver(_drv: &'static dyn Driver) {}
