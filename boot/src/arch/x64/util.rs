
#[repr(align(8))]
pub struct BigBuf<const N: usize> {
    data: [u8; N],
}
impl<const N: usize> BigBuf<N> {
    pub const fn new() -> Self {
        Self { data: [0; N] }
    }
    
    pub fn inner_mut(&mut self) -> &mut [u8; N] {
        &mut self.data
    }
}
