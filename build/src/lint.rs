
use crate::{LintOpts, Project};

use duct::cmd;

pub fn lint_main(opts: &LintOpts, config: &[Project]) {
    for proj in config {
        println!("==> Linting {}", proj.name);

        let mut args: Vec<String> = vec![
            "clippy".into(),
            "-p".into(),
            proj.cargo_name.clone(),
        ];
        if opts.build.release {
            args.push("--release".into());
        }
        args.push("--target".into());
        args.push(proj.flags.target.as_ref().unwrap().to_string_lossy().into_owned());
        args.extend(proj.flags.flags.iter().cloned());
        args.extend(opts.build.cargo_args.iter().cloned());
        
        args.push("--".into());
        args.extend(opts.clippy_args.iter().cloned().flatten());

        cmd("cargo", args).run().unwrap();
    }
}