use chos_lib::{arch::mm::{Flush, FrameSize4K, PageTable}, mm::Mapper};

pub struct ChosMapper<'a> {
    p4: &'a mut PageTable,
}
