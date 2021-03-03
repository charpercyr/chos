
use std::error::Error;
use std::env;
use std::fmt;
use std::process::{Command};

#[derive(Debug, Copy, Clone)]
struct RunError<'a> {
    cmd: &'a str,
    status: i32,
}

impl fmt::Display for RunError<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Failed to run '{}' ({})", self.cmd, self.status)
    }
}

impl Error for RunError<'_> {}

fn main() -> Result<(), Box<dyn Error>> {
    let arch = env::var("CARGO_CFG_TARGET_ARCH")?;
    let outdir = env::var("OUT_DIR")?;

    let library_name = "acpica";
    let library = format!("{}/lib{}.a", outdir, library_name);

    let make_result = Command::new("make")
        .env("ARCH", arch)
        .env("BINDIR", &outdir)
        .env("LIB", library)
        .status()?;
    if !make_result.success() {
        let code = make_result.code().unwrap_or(1);
        return Err(Box::new(RunError {
            cmd: "make",
            status: code,
        }));
    }

    println!("cargo:rerun-if-changed=Makefile");
    println!("cargo:rustc-link-lib={}", library_name);
    println!("cargo:rustc-link-search={}", outdir);

    Ok(())
}
