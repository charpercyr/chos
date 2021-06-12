
use crate::{BuildOpts, Project};

use duct::cmd;

fn validate_config(config: &[Project]) {
    for proj in config {
        assert!(proj.flags.target.is_some(), "Target must be set");
        assert!(proj.flags.linker_script.is_some(), "Linker script must be set");
    }
}

pub fn build_main(opts: &BuildOpts, config: &[Project]) {
    validate_config(config);
    for proj in config {
        println!("==> Building {}", proj.name);

        let mut args: Vec<String> = vec![
            "rustc".into(),
            "-p".into(),
            proj.cargo_name.clone(),
        ];
        if opts.release {
            args.push("--release".into());
        }
        args.push("--target".into());
        args.push(proj.flags.target.as_ref().unwrap().to_string_lossy().into_owned());
        args.extend(proj.flags.flags.iter().cloned());

        args.push("--".into());
        args.push(format!("-Clink-args=-T{}", proj.flags.linker_script.as_ref().unwrap().to_string_lossy()));
        args.extend(proj.flags.rustc_flags.iter().cloned());
        
        cmd("cargo", args).run().unwrap();
    }
}
