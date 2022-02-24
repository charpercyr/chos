use duct::cmd;
use tempfile::Builder;

use crate::build::build_main;
use crate::consts::DEPLOY_DEFAULT_SIZE;
use crate::deploy::deploy;
use crate::opts::RunOpts;
use crate::util::display_cmd_hook;
use crate::Project;
const KERNEL_EXIT_SUCCESS: i32 = 33;

pub fn run_main(opts: &RunOpts, projects: &[Project]) {
    if opts.build.arch != "x86_64" {
        panic!("Run not supported for {}", opts.build.arch);
    }
    if !opts.no_build {
        build_main(&opts.build, projects);
    }
    let imgfile = Builder::new()
        .prefix("chos")
        .suffix(".img")
        .tempfile()
        .unwrap();
    deploy(
        imgfile.path(),
        projects,
        opts.build.release,
        DEPLOY_DEFAULT_SIZE,
    )
    .unwrap();

    let smp = format!("{}", opts.smp);
    let imgfile_str = imgfile.path().to_string_lossy();
    // let imgdrive_args = format!("if=ide,file={}", );
    let mut args = vec![
        "-bios", "/usr/share/ovmf/x64/OVMF.fd",
        "-hda", &imgfile_str,
        "-m", &opts.mem,
        "-smp", &smp,
        "-machine", "q35",
        "-cpu", "Skylake-Client-v3",
        "-device", "isa-debug-exit,iobase=0xf4,iosize=0x4",
        "-display", "none",
        "-serial", "stdio",
    ];

    if opts.debug {
        args.push("-s");
        args.push("-S");
    }

    let qemu = cmd(format!("qemu-system-{}", opts.build.arch), args)
        .before_spawn(display_cmd_hook)
        .unchecked()
        .stderr_null()
        .run()
        .unwrap();
    if qemu.status.code().unwrap() != KERNEL_EXIT_SUCCESS {
        panic!("Qemu failed with exit code {}", qemu.status.code().unwrap());
    }
}
