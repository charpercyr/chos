use std::path::{Path, PathBuf};
use std::process::Command;
use std::str::FromStr;

pub struct ErrorGuard<F: FnOnce()> {
    f: Option<F>,
}

impl<F: FnOnce()> ErrorGuard<F> {
    pub fn new(on_err: F) -> Self {
        Self { f: Some(on_err) }
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

#[derive(Debug, Clone)]
pub enum Target {
    Builtin(String),
    File(PathBuf),
}

impl Target {
    pub fn from_base_str(base: &Path, s: &str) -> Target {
        const FILE_PREFIX: &'static str = "file:";
        if s.starts_with(FILE_PREFIX) {
            Target::File(base.join(&s[FILE_PREFIX.len()..]))
        } else {
            Target::Builtin(s.into())
        }
    }

    pub fn name(&self) -> &str {
        match self {
            Self::Builtin(name) => name,
            Self::File(path) => path
                .file_prefix()
                .expect("Target file should have a name")
                .to_str()
                .expect("Invalid target name"),
        }
    }

    pub fn value(&self) -> &str {
        match self {
            Self::Builtin(name) => name,
            Self::File(path) => path.to_str().expect("Invalid target name"),
        }
    }
}
