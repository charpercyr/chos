use super::reg;

pub struct Lint<'a> {
    reg: &'a mut reg::LocalInterrupt,
}
