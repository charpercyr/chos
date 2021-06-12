
use std::process::{Command};

pub struct ErrorGuard<F: FnOnce()> {
    f: Option<F>,
}

impl<F: FnOnce()> ErrorGuard<F> {
    pub fn new(on_err: F) -> Self {
        Self {
            f: Some(on_err),
        }
    }

    pub fn defuse(mut self) {
        self.f = None
    }
}

impl<F: FnOnce()> Drop for ErrorGuard<F> {
    fn drop(&mut self) {
        if let Some(f) = self.f.take() {
            f();
        }
    }
}


pub fn display_cmd(cmd: &Command) {
    print!("> {} ", cmd.get_program().to_string_lossy());
    for s in cmd.get_args() {
        let s = s.to_string_lossy();
        if s.contains(' ') {
            print!("\"{}\" ", s);
        } else {
            print!("{} ", s);
        }
    }
    println!();
}

pub fn display_cmd_hook(cmd: &mut Command) -> std::io::Result<()> {
    Ok(display_cmd(cmd))
}