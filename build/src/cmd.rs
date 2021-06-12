
use std::process::{Command, ExitStatus};

pub fn cargo() -> Command {
    Command::new("cargo")
}

pub fn display_cmd(cmd: &Command) {
    for (k, v) in cmd.get_envs() {
        let k = k.to_string_lossy();
        if k.contains(' ') {
            print!("\"{}\"=", k);
        } else {
            print!("{}=", k);
        }
        if let Some(v) = v {
            let v = v.to_string_lossy();
            if v.contains(' ') {
                print!("\"{}\" ", v);
            } else {
                print!("{} ", v);
            }
        }
    }
    print!("{} ", cmd.get_program().to_string_lossy());
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

pub fn check_status(status: ExitStatus) {
    if !status.success() {
        panic!("Failed with {}", status);
    }
}

pub fn status(cmd: &mut Command) {
    display_cmd(&cmd);
    check_status(cmd.status().unwrap());
}