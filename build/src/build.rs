
use crate::{BuildOpts, Project};

fn validate_config(config: &Vec<Project>) {
    for proj in config {
        assert!(proj.flags.target.is_some(), "Target must be set");
        assert!(proj.flags.linker_script.is_some(), "Linker script must be set");
    }
}

pub fn build_main(opts: &BuildOpts, config: &Vec<Project>) {
    validate_config(config);
    for proj in config {
        println!("==> Building {}", proj.name);
        let mut cmd = crate::cmd::cargo();
        cmd.arg("rustc");
        cmd.arg("-p").arg(&proj.cargo_name);
        if opts.release {
            cmd.arg("--release");
        }
        cmd.arg("--target").arg(proj.flags.target.as_ref().unwrap());
        cmd.args(&proj.flags.flags);

        cmd.arg("--");
        cmd.arg(format!("-Clink-args=-T{}", proj.flags.linker_script.as_ref().unwrap().to_string_lossy()));
        cmd.args(&proj.flags.rustc_flags);
        
        crate::cmd::status(&mut cmd);
    }
}
