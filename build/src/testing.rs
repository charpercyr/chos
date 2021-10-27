use crate::{opts::TestOpts, project::Project, util::display_cmd_hook};

use duct::cmd;

use std::io;

const LIBS: &'static [&'static str] = &["chos-lib"];

fn test_package(name: &str) -> io::Result<()> {
    cmd!("cargo", "test", "-p", name,)
        .before_spawn(display_cmd_hook)
        .run()?;
    Ok(())
}

pub fn test_main(opts: &TestOpts, _: &[Project]) {
    // TODO run kernel in testing mode
    // build_main(&opts.build, config)
    if let Some(pkgs) = &opts.packages {
        for p in pkgs {
            test_package(p).unwrap();
        }
    } else {
        for &p in LIBS {
            test_package(p).unwrap();
        }
    }
}
