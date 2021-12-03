use crate::{opts::TestOpts, project::Project, util::display_cmd_hook};

use duct::cmd;

use std::io;

const LIBS: &'static [&'static str] = &["chos-lib"];

fn test_package(name: &str, opts: &TestOpts) -> io::Result<()> {
    let mut args = vec!["test", "-p", name, "--all-features"];
    args.extend(opts.build.cargo_args.iter().map::<&str, _>(|i| i));
    cmd("cargo", args)
        .before_spawn(display_cmd_hook)
        .run()?;
    Ok(())
}

pub fn test_main(opts: &TestOpts, _: &[Project]) {
    // TODO run kernel in testing mode
    // build_main(&opts.build, config)
    if let Some(pkgs) = &opts.packages {
        for p in pkgs {
            test_package(p, opts).unwrap();
        }
    } else {
        for &p in LIBS {
            test_package(p, opts).unwrap();
        }
    }
}
