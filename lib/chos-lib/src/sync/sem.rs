
pub trait Sem {
    fn with_count(count: usize) -> Self;

    fn wait_count(&self, count: usize);
    fn signal_count(&self, count: usize);

    fn wait(&self) {
        self.wait_count(1)
    }
    fn signal(&self) {
        self.signal_count(1)
    }
}
