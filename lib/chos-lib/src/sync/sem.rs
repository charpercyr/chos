pub trait Sem {
    fn zero() -> Self
    where
        Self: Sized,
    {
        Self::with_count(0)
    }

    fn with_count(count: usize) -> Self
    where
        Self: Sized;

    fn wait_count(&self, count: usize);
    fn signal_count(&self, count: usize);

    fn wait(&self) {
        self.wait_count(1)
    }
    fn signal(&self) {
        self.signal_count(1)
    }
}

pub trait TrySem: Sem {
    fn try_wait_count(&self, count: usize) -> bool;

    fn try_wait(&self) -> bool {
        self.try_wait_count(1)
    }

    fn try_wait_count_tries(&self, count: usize, tries: usize) -> bool {
        for _ in 0..tries {
            if self.try_wait_count(count) {
                return true;
            }
        }
        false
    }

    fn try_wait_tries(&self, tries: usize) -> bool {
        self.try_wait_count_tries(1, tries)
    }
}
