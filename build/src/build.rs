use duct::cmd;

use crate::opts::*;
use crate::util::{display_cmd_hook, Target};
use crate::Project;

pub fn build_main(opts: &BuildOpts, projects: &[Project]) {
    for proj in projects {
        println!("==> Building {}", proj.name);

        let mut args: Vec<String> = vec!["rustc".into(), "-p".into(), proj.name.clone()];
        if opts.release {
            args.push("--release".into());
        }
        args.push("--target".into());
        args.push(proj.target.value().into());
        args.extend(proj.flags.flags.clone());
        args.extend(opts.cargo_args.iter().cloned());

        args.push("--".into());
        if let Some(linker_script) = proj.flags.linker_script.as_deref() {
            args.push(format!(
                "-Clink-args=-T{}",
                proj.path.join(linker_script).to_string_lossy()
            ));
        }
        args.extend(proj.flags.rustc_flags.clone());
        args.extend(opts.rustc_args.iter().cloned());

        cmd("cargo", args)
            .before_spawn(display_cmd_hook)
            .run()
            .unwrap();
    }
}
