use std::fs::OpenOptions;
use std::io::Write;
use std::path::Path;

use duct::cmd;

use crate::opts::{DriverOpts, NewDriverOpts};
use crate::util::display_cmd_hook;

const DRIVER_CARGO_TOML: &'static str = include_str!("../static/driver-Cargo.toml");
const DRIVER_LIB_RS: &'static str = include_str!("../static/driver-lib.rs");

const NAME_KEY: &'static str = "%NAME%";

fn get_cargo_toml(name: &str) -> String {
    DRIVER_CARGO_TOML.replace(NAME_KEY, name)
}

fn get_lib_rs(_name: &str) -> String {
    DRIVER_LIB_RS.into()
}

fn new_driver(opts: &NewDriverOpts) {
    if opts.static_ && opts.initrd {
        eprintln!("Cannot be both '--static' and '--initrd'");
        panic!();
    }
    let path = Path::new("drivers").join(&opts.category).join(&opts.name);
    cmd!("cargo", "new", &path, "--lib")
        .before_spawn(display_cmd_hook)
        .run()
        .unwrap();

    let project_name = format!("chos-{}-{}", opts.category, opts.name);
    let cargo_toml_content = get_cargo_toml(&project_name);
    let lib_rs_content = get_lib_rs(&project_name);

    let mut open_opts = OpenOptions::new();
    open_opts.write(true);
    open_opts.truncate(true);

    let mut cargo_toml = open_opts
        .open(path.join("Cargo.toml"))
        .expect("Could not open Cargo.toml");
    cargo_toml
        .write_all(cargo_toml_content.as_bytes())
        .expect("Could not write Cargo.toml");

    let mut lib_rs = open_opts
        .open(path.join("src/lib.rs"))
        .expect("Could not open src/lib.rs");
    lib_rs
        .write_all(lib_rs_content.as_bytes())
        .expect("Could not write src/lib.rs");
}

pub fn driver_main(opts: &DriverOpts) {
    match opts {
        DriverOpts::New(opts) => new_driver(opts),
    }
}
