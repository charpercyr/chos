use super::Device;

pub struct BlockDeviceAttrs {
    pub block_size: u64,
    pub block_count: u64,
}

pub trait BlockDevice: Device {
    fn attributes(&self) -> &BlockDeviceAttrs;
}
