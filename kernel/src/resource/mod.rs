use chos_lib::{ReadOnly, ReadWrite};

use crate::fs::buf::BufOwn;
use crate::fs::{self};

pub trait Resource {
    fn close(&self);

    fn file(&self) -> Option<&dyn File>;
}

pub trait File {
    fn read(&self, buf: BufOwn<ReadWrite>, result: fs::Sender<(usize, BufOwn<ReadWrite>)>);
    fn write(&self, buf: BufOwn<ReadOnly>, result: fs::Sender<(usize, BufOwn<ReadOnly>)>);
}

pub trait Directory {
    fn list(&self);
}
