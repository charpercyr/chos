
pub trait Sem {
    fn new_with_count(count: usize) -> Self;

    fn wait(&self);
    fn signal(&self);

    fn wait_count(&self, count: usize) {
        for _ in 0..count {
            self.wait();
        }
    }

    fn signal_count(&self, count: usize) {
        for _ in 0..count {
            self.signal();
        }
    }
}
