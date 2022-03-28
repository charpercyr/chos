use chos_lib::sync::{Sem, SpinSem};

pub mod sem;

pub struct SchedSem {
    raw_sem: SpinSem,
}

impl Sem for SchedSem {
    fn with_count(count: usize) -> Self {
        Self {
            raw_sem: SpinSem::with_count(count),
        }
    }

    fn wait_count(&self, count: usize) {

    }

    fn signal_count(&self, count: usize) {

    }
}
