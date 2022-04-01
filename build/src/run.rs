
use crate::{Project, RunOpts, build::build_main};

use duct::cmd;

use tempfile::Builder;

const KERNEL_EXIT_SUCCESS: i32 = 33;

pub fn run_main(opts: &RunOpts, config: &[Project]) {
    if opts.build.arch != "x86_64" {
        panic!("Run not supported for {}", opts.build.arch);
    }
    if !opts.no_build {
        build_main(&opts.build, config);
    }
    let imgfile = Builder::new().prefix("chos").suffix(".img").tempfile().unwrap();
    crate::deploy(imgfile.path(), config, opts.build.release, crate::DEPLOY_DEFAULT_SIZE).unwrap();

    let smp = format!("{}", opts.smp);
    let imgfile_str = imgfile.path().to_string_lossy();
    let qemu = format!("qemu-system-{}", opts.build.arch);
    let (machine, cpu) = match !opts.no_kvm {
        true => ("q35,accel=kvm", "host"),
        false => ("q35", "Skylake-Client"),
    };
    let (display, serial) = match opts.curses {
        true => ("curses", "none"),
        false => ("none", "stdio"),
    };
    let mut args = vec![
        &*qemu,
        "-m", &opts.mem,
        "-smp", &smp,
        "-machine", machine,
        "-cpu", cpu,
        "-device", "isa-debug-exit,iobase=0xf4,iosize=0x4",
        "-display", display,
        "-serial", serial,
        &*imgfile_str,
        "-D", "target/qemu.log",
        "-d", "guest_errors",
    ];

    if opts.debug {
        args.push("-s");
        args.push("-S");
    }

    let qemu = cmd("sudo", args)
        .before_spawn(crate::display_cmd_hook)
        .unchecked()
        // .stderr_null()
        .run()
        .unwrap();
    if qemu.status.code().unwrap() != KERNEL_EXIT_SUCCESS {
        panic!("Qemu failed with exit code {}", qemu.status.code().unwrap());
    }
}
